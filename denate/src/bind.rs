use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Condvar, Mutex};

use background_hang_monitor::HangMonitorRegister;
use compositing_traits::{
    CompositorMsg, CompositorProxy, CompositorReceiver,
    FontToCompositorMsg, ForwardedToCompositorMsg,
};
use crossbeam_channel::unbounded;
use embedder_traits::{EmbedderProxy, EmbedderReceiver};
use euclid::{Scale, Size2D};
use gfx::font_cache_thread::FontCacheThread;
use ipc_channel::ipc;
use layout_thread_2020::LayoutThread;
use layout_traits::LayoutThreadFactory;
use metrics::PaintTimeMetrics;
use msg::constellation_msg::{PipelineId, TopLevelBrowsingContextId};
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

fn main() {
    let layout_pair = unbounded::<Msg>();
    //let layout_chan = layout_pair.0.clone();

    let pipeline_id = PipelineId::new();
    let time_profiler_chan = profile_time::Profiler::create(&None, None);
    let mem_profiler_chan = profile_mem::Profiler::create(None);
    let (webrender_image_ipc_sender, _webrender_image_ipc_receiver) =
        ipc::channel().expect("ipc channel failure");
    let image_cache = Arc::new(ImageCacheImpl::new(net_traits::WebrenderIpcSender::new(
        webrender_image_ipc_sender,
    )));

    let (script_chan, _) = ipc::channel().expect("ipc channel failure");
    let (_, pipeline_port) = ipc::channel().expect("ipc channel failure");

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

    let webrender_api_sender = script_traits::WebrenderIpcSender::new(webrender_ipc_sender);

    let event_loop_waker = Box::new(HeadlessEventLoopWaker(Arc::new((
        Mutex::new(false),
        Condvar::new(),
    ))));
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

    let font_cache_thread = FontCacheThread::new(
        public_resource_threads.sender(),
        Box::new(FontCacheWR(compositor_proxy.clone())),
    );

    let url = ServoUrl::from_url(Url::parse("about:blank").unwrap());

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
            time_profiler_chan.clone(),
            constellation_chan_sender2.clone(),
            script_chan.clone(),
            url.clone(),
        ),
        Arc::new(AtomicBool::new(false)),
        WindowSizeData {
            initial_viewport: Size2D::new(800.0f32, 600.0f32),
            device_pixel_ratio: Scale::new(1.0),
        },
    );
}
