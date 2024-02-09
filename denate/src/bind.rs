use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Condvar, Mutex};

use background_hang_monitor::HangMonitorRegister;
use compositing_traits::{
    CompositorMsg, CompositorProxy, CompositorReceiver, FontToCompositorMsg,
    ForwardedToCompositorMsg,
};
use crossbeam_channel::unbounded;
use embedder_traits::{EmbedderProxy, EmbedderReceiver};
use euclid::{Scale, Size2D};
use gfx::font_cache_thread::FontCacheThread;
use ipc_channel::ipc;
use ipc_channel::router::ROUTER;
use layout_thread_2020::LayoutThread;
use layout_traits::LayoutThreadFactory;
use metrics::PaintTimeMetrics;
use msg::constellation_msg::{
    PipelineId, PipelineNamespace, PipelineNamespaceId, PipelineNamespaceInstaller,
    TopLevelBrowsingContextId,
};
use net::image_cache::ImageCacheImpl;
use net::resource_thread::new_resource_threads;
use net_traits::image_cache::ImageCache;
use net_traits::IpcSend;
use profile::{mem as profile_mem, time as profile_time};
use script_layout_interface::message::Msg;
use script_traits::WindowSizeData;
use servo_url::ServoUrl;
use url::Url;
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
    let layout_pair = unbounded::<Msg>();
    //let layout_chan = layout_pair.0.clone();

    let (namespace_request_sender, _namespace_request_receiver) =
        ipc::channel().expect("ipc channel failure");
    println!("setting up pipeline namespace");
    let mut pipeline_namespace = PipelineNamespaceInstaller::new();
    println!("setting sender");
    pipeline_namespace.set_sender(namespace_request_sender);
    println!("installing namespace");
    PipelineNamespace::install(PipelineNamespaceId(1));
    println!("setting up profiler");
    let pipeline_id = PipelineId::new();
    let time_profiler_chan = profile_time::Profiler::create(&None, None);
    let mem_profiler_chan = profile_mem::Profiler::create(None);

    let (webrender_image_ipc_sender, _webrender_image_ipc_receiver) =
        ipc::channel().expect("ipc channel failure");

    println!("preparing webrender for image cache");
    let web_render_ipc_sender = net_traits::WebrenderIpcSender::new(webrender_image_ipc_sender);
    println!("creating image cache");
    let image_cache = Arc::new(ImageCacheImpl::new(web_render_ipc_sender));

    println!("creating script channels");
    let (script_chan, _) = ipc::channel().expect("ipc channel failure");
    let (_, pipeline_port) = ipc::channel().expect("ipc channel failure");

    println!("creating background hang monitor register");
    let (constellation_chan_sender, _constellation_chan_receiver) =
        ipc::channel().expect("ipc channel failure");
    let (constellation_chan_sender2, _constellation_chan_receiver2) =
        ipc::channel().expect("ipc channel failure");
    let (_control_sender, control_receiver) = ipc::channel().expect("ipc channel failure");
    let background_hang_monitor_register =
        HangMonitorRegister::init(constellation_chan_sender.clone(), control_receiver, false);

    let (layout_ipc_sender, _layout_ipc_receiver) = ipc::channel().expect("ipc channel failure");

    let (webrender_ipc_sender, _webrender_ipc_receiver) =
        ipc::channel().expect("ipc channel failure");

    println!("creating webrender ipc sender");
    let webrender_api_sender = script_traits::WebrenderIpcSender::new(webrender_ipc_sender);

    println!("setting up event loop waker");
    let event_loop_waker = Box::new(HeadlessEventLoopWaker(Arc::new((
        Mutex::new(false),
        Condvar::new(),
    ))));
    println!("creating embedder");
    let (embedder_sender, embedder_receiver) = unbounded();
    let (embedder_proxy, _embedder_receiver) = (
        EmbedderProxy {
            sender: embedder_sender,
            event_loop_waker: event_loop_waker.clone(),
        },
        EmbedderReceiver {
            receiver: embedder_receiver,
        },
    );
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
    let (compositor_sender, compositor_receiver) = unbounded();
    let (compositor_proxy, _compositor_receiver) = (
        CompositorProxy {
            sender: compositor_sender,
            event_loop_waker,
        },
        CompositorReceiver {
            receiver: compositor_receiver,
        },
    );

    println!("creating font thread");
    let font_cache_thread = FontCacheThread::new(
        public_resource_threads.sender(),
        Box::new(FontCacheWR(compositor_proxy.clone())),
    );

    println!("creating blank url");
    let url = ServoUrl::from_url(Url::parse("about:blank").unwrap());

    println!("creating layout thread");
    LayoutThread::create(
        pipeline_id,
        TopLevelBrowsingContextId::new(),
        url.clone(),
        false,
        layout_pair,
        pipeline_port,
        background_hang_monitor_register,
        layout_ipc_sender,
        script_chan.clone(),
        image_cache,
        font_cache_thread,
        time_profiler_chan.clone(),
        mem_profiler_chan,
        webrender_api_sender,
        PaintTimeMetrics::new(
            pipeline_id,
            time_profiler_chan,
            constellation_chan_sender2,
            script_chan,
            url,
        ),
        Arc::new(AtomicBool::new(false)),
        WindowSizeData {
            initial_viewport: Size2D::new(800.0f32, 600.0f32),
            device_pixel_ratio: Scale::new(1.0),
        },
    );
    println!("DONE OMG");
}
