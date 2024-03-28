use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};

use compositing_traits::{
    CompositorMsg, CompositorProxy, FontToCompositorMsg, ForwardedToCompositorMsg,
};
use crossbeam_channel::unbounded;
use embedder_traits::EmbedderProxy;
use euclid::{Point2D, Rect, Scale, Size2D};
use fxhash::FxBuildHasher;
use gfx::font_cache_thread::FontCacheThread;
use ipc_channel::ipc;
use layout_thread_2020::LayoutFactoryImpl;
use libc::c_void;
use metrics::PaintTimeMetrics;
use msg::constellation_msg::{
    PipelineId, PipelineNamespace, PipelineNamespaceId, PipelineNamespaceInstaller,
};
use net::image_cache::ImageCacheImpl;
use net::resource_thread::new_resource_threads;
use net_traits::image_cache::ImageCache;
use net_traits::IpcSend;
use parking_lot::RwLock;
use profile::{mem as profile_mem, time as profile_time};
use script_layout_interface::message::{Msg, Reflow, ReflowGoal, ScriptReflow};
use script_layout_interface::{LayoutConfig, LayoutFactory, TrustedNodeAddress};
use script_traits::WindowSizeData;
use servo_url::ServoUrl;
use style::animation::DocumentAnimationSet;
use url::Url;
use webrender_api::units::Au;
use webrender_api::{FontInstanceKey, FontKey};

use crate::events_loop::HeadlessEventLoopWaker;

struct FontCacheWR(CompositorProxy);

impl gfx_traits::WebrenderApi for FontCacheWR {
    fn add_font_instance(&self, font_key: FontKey, size: f32) -> FontInstanceKey {
        let (sender, receiver) = unbounded();
        let _ = self
            .0
            .send(CompositorMsg::Forwarded(ForwardedToCompositorMsg::Font(
                FontToCompositorMsg::AddFontInstance(font_key, size, sender),
            )));
        receiver.recv().unwrap()
    }
    fn add_font(&self, data: gfx_traits::FontData) -> FontKey {
        let (sender, receiver) = unbounded();
        let _ = self
            .0
            .send(CompositorMsg::Forwarded(ForwardedToCompositorMsg::Font(
                FontToCompositorMsg::AddFont(data, sender),
            )));
        receiver.recv().unwrap()
    }
}

pub fn main() {
    println!("main");
    let namespace_request_chan = ipc::channel().expect("ipc channel failure");
    println!("setting up pipeline namespace");
    let mut pipeline_namespace = PipelineNamespaceInstaller::default();
    println!("setting sender");
    pipeline_namespace.set_sender(namespace_request_chan.0);
    println!("installing namespace");
    PipelineNamespace::install(PipelineNamespaceId(1));
    println!("setting up profiler");
    let pipeline_id = PipelineId::new();
    let time_profiler_chan = profile_time::Profiler::create(&None, None);
    let mem_profiler_chan = profile_mem::Profiler::create(None);

    let webrender_image_channel = ipc::channel().expect("ipc channel failure");

    println!("preparing webrender for image cache");
    let webrender_sender = net_traits::WebrenderIpcSender::new(webrender_image_channel.0);
    println!("creating image cache");
    let image_cache = Arc::new(ImageCacheImpl::new(webrender_sender));

    println!("creating script channels");
    let script_chan = ipc::channel().expect("ipc channel failure");

    println!("creating background hang monitor register");
    let constellation_chan = ipc::channel().expect("ipc channel failure");

    println!("creating webrender ipc sender");
    let webrender_chan = ipc::channel().expect("ipc channel failure");
    let webrender_api_sender = script_traits::WebrenderIpcSender::new(webrender_chan.0);

    println!("setting up event loop waker");
    let event_loop_waker = Box::new(HeadlessEventLoopWaker(Arc::new((
        Mutex::new(false),
        Condvar::new(),
    ))));
    println!("creating embedder");
    let embedder_chan = unbounded();
    let embedder_proxy = EmbedderProxy {
        sender: embedder_chan.0,
        event_loop_waker: event_loop_waker.clone(),
    };
    println!("setting up resource thread");
    let (public_resource_threads, _private_resource_threads) = new_resource_threads(
        "".into(),
        None,
        time_profiler_chan.clone(),
        mem_profiler_chan.clone(),
        embedder_proxy.clone(),
        None,
        None,
        true,
    );

    println!("setting up compositor proxy");
    let compositor_chan = unbounded();
    let compositor_proxy = CompositorProxy {
        sender: compositor_chan.0,
        event_loop_waker,
    };

    println!("creating font thread");
    let font_cache_thread = FontCacheThread::new(
        public_resource_threads.sender(),
        Box::new(FontCacheWR(compositor_proxy.clone())),
    );

    println!("creating blank url");
    let url = ServoUrl::from_url(Url::parse("about:blank").unwrap());

    println!("creating layout thread");
    let mut layout = LayoutFactoryImpl().create(LayoutConfig {
        id: pipeline_id,
        url: url.clone(),
        is_iframe: false,
        constellation_chan: constellation_chan.0.clone(),
        script_chan: script_chan.0.clone(),
        image_cache,
        font_cache_thread,
        time_profiler_chan: time_profiler_chan.clone(),
        webrender_api_sender,
        paint_time_metrics: PaintTimeMetrics::new(
            pipeline_id,
            time_profiler_chan,
            constellation_chan.0,
            script_chan.0,
            url.clone(),
            0,
        ),
        window_size: WindowSizeData {
            initial_viewport: Size2D::new(800.0f32, 600.0f32),
            device_pixel_ratio: Scale::new(1.0),
        },
    });

    let reflow_complete_sender = unbounded();
    let reflow = ScriptReflow {
        reflow_info: Reflow {
            page_clip_rect: Rect {
                origin: Point2D::new(Au(0), Au(0)),
                size: Size2D::new(Au(500), Au(500)),
            },
        },
        document: TrustedNodeAddress(0 as *const c_void),
        dirty_root: None,
        stylesheets_changed: false,
        window_size: WindowSizeData {
            initial_viewport: Size2D::new(500.0, 500.0),
            device_pixel_ratio: Scale::new(1.0),
        },
        origin: servo_url::ImmutableOrigin::Tuple(
            "http".to_owned(),
            url::Host::Domain("quox.dev".to_owned()),
            80,
        ),
        reflow_goal: ReflowGoal::Full,
        script_join_chan: reflow_complete_sender.0,
        dom_count: 0,
        pending_restyles: vec![],
        animation_timeline_value: 0.0,
        animations: DocumentAnimationSet {
            sets: servo_arc::Arc::new(RwLock::new(HashMap::with_hasher(FxBuildHasher::default()))),
        },
    };
    layout.process(Msg::Reflow(reflow));

    println!("DONE OMG");
}
