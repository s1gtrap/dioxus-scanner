#![allow(non_snake_case)]

use std::{marker::PhantomData, rc::Rc};

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;

pub use rxing::RXingResult;
pub use web_sys::DomException;

#[derive(Clone)]
pub struct StaticCallback<T> {
    inner: Rc<RefCell<Box<dyn FnMut(T)>>>,
}

impl<F, T> From<F> for StaticCallback<T>
where
    F: FnMut(T) + 'static,
{
    fn from(f: F) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Box::new(f))),
        }
    }
}

#[derive(Props)]
pub struct ScannerProps<'a> {
    #[props(into)]
    handlescan: Option<StaticCallback<rxing::RXingResult>>,
    #[props(into)]
    handleerror: Option<StaticCallback<web_sys::DomException>>,
    #[props(default = PhantomData)]
    phantom: PhantomData<&'a ()>,
}

async fn get_stream() -> Result<web_sys::MediaStream, web_sys::DomException> {
    let window = web_sys::window().unwrap();

    let media_devices = window.navigator().media_devices().unwrap();

    let mut constraints = web_sys::MediaStreamConstraints::new();
    constraints.video(&wasm_bindgen::JsValue::TRUE);
    log::info!("waiting for permission..");
    wasm_bindgen_futures::JsFuture::from(
        media_devices
            .get_user_media_with_constraints(&constraints)
            .map_err(|s| s.dyn_into::<web_sys::DomException>().unwrap())?,
    )
    .await
    .map(|s| s.dyn_into().unwrap())
    .map_err(|s| s.dyn_into().unwrap())
}

fn scan_barcode(video: &web_sys::HtmlVideoElement) -> Option<rxing::RXingResult> {
    // FIXME: hacky way to skip empty images
    if video.video_width() == 0 {
        return None;
    }

    // FIXME: inefficient to reconstruct the canvas
    let canvas = web_sys::OffscreenCanvas::new(video.video_width(), video.video_height()).unwrap();
    let ctx: web_sys::OffscreenCanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();

    ctx.draw_image_with_html_video_element_and_dw_and_dh(
        video,
        0.0,
        0.0,
        canvas.width() as _,
        canvas.height() as _,
    )
    .unwrap();

    let data = ctx
        .get_image_data(0.0, 0.0, canvas.width() as _, canvas.height() as _)
        .unwrap();
    let data = (*data.data()).to_vec();

    use rxing::common::HybridBinarizer;
    use rxing::{BinaryBitmap, BufferedImageLuminanceSource, Reader};
    let mut image = BinaryBitmap::new(HybridBinarizer::new(BufferedImageLuminanceSource::new(
        image::DynamicImage::from(
            image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                canvas.width() as _,
                canvas.height() as _,
                data,
            )
            .unwrap(),
        ),
    )));

    let mut reader = rxing::qrcode::QRCodeReader;

    reader.decode(&mut image).ok()
}

fn frame_callback_loop<F>(mut f: F)
where
    F: FnMut() + 'static,
{
    #[wasm_bindgen]
    extern "C" {
        // FIXME: should probably use requestVideoFrameCallback
        fn requestAnimationFrame(closure: &Closure<dyn FnMut()>) -> u32;
    }

    let closure = Rc::new(RefCell::new(None));
    *closure.borrow_mut() = Some({
        let closure = closure.clone();
        Closure::<dyn FnMut()>::new(move || {
            f();
            requestAnimationFrame(closure.borrow().as_ref().unwrap());
        })
    });

    requestAnimationFrame(closure.borrow().as_ref().unwrap());
}

pub fn Scanner<'a>(cx: Scope<'a, ScannerProps<'a>>) -> Element<'a> {
    let id = format!(
        "dx-scanner-{:032x}",
        <rand::rngs::ThreadRng as rand::Rng>::gen::<u128>(&mut rand::thread_rng()),
    ); // is generating ids even that smart?

    let stream = use_state(cx, || None::<web_sys::MediaStream>);

    // getUserMedia
    use_future(cx, (stream,), move |(stream,)| {
        let handleerror = cx.props.handleerror.clone();
        async move {
            if stream.get().is_none() {
                match get_stream().await {
                    Ok(s) => stream.set(Some(s)),
                    Err(e) => {
                        if let Some(handler) = handleerror {
                            (handler.inner.borrow_mut())(e);
                        }
                    }
                }
            }
        }
    });

    use_effect(cx, (stream,), |(stream,)| {
        let id = id.clone();
        let handlescan = cx.props.handlescan.clone();
        async move {
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();

            if let Some(stream) = stream.get() {
                // set srcObject
                let video: web_sys::HtmlVideoElement =
                    document.get_element_by_id(&id).unwrap().dyn_into().unwrap();
                video.set_src_object(Some(stream));

                frame_callback_loop(move || {
                    if let Some(handler) = &handlescan {
                        if let Some(res) = scan_barcode(&video) {
                            (handler.inner.borrow_mut())(res);
                        }
                    }
                })
            }
        }
    });

    render! {
        if stream.get().is_some() {
            rsx! {
                video {
                    id: "{id}",
                    autoplay: true,
                }
            }
        } else {
            rsx! {
                "loading.."
            }
        }
    }
}
