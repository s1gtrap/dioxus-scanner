#![allow(non_snake_case)]

use std::{marker::PhantomData, rc::Rc};

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
pub struct StaticCallback<T> {
    inner: Rc<RefCell<Box<dyn FnMut(T)>>>,
}

impl<F, T> From<F> for StaticCallback<T>
where
    F: FnMut(T) -> () + 'static,
{
    fn from(mut f: F) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Box::new(move |input| f(input)))),
        }
    }
}

#[derive(Props)]
pub struct ScannerProps<'a> {
    #[props(into)]
    cb: Option<StaticCallback<rxing::RXingResult>>,
    #[props(default = PhantomData)]
    phantom: PhantomData<&'a ()>,
}

pub fn Scanner<'a>(cx: Scope<'a, ScannerProps<'a>>) -> Element<'a> {
    let id = format!(
        "dx-barcode-feed-{:032x}",
        <rand::rngs::ThreadRng as rand::Rng>::gen::<u128>(&mut rand::thread_rng()),
    );

    use_future(cx, (), {
        let id = id.clone();
        move |_| {
            let handle_file = cx.props.cb.clone();

            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();

            // TODO: should probably be requestVideoFrameCallback
            let f = Rc::new(RefCell::new(None));

            let request_animation_frame = {
                let window = window.clone();
                move |f: &Closure<dyn FnMut()>| {
                    window
                        .request_animation_frame(f.as_ref().unchecked_ref())
                        .expect("should register `requestAnimationFrame` OK");
                }
            };
            let request_animation_frame2 = request_animation_frame.clone();
            let g = f.clone();

            async move {
                let media_devices = window.navigator().media_devices().unwrap();

                let mut constraints = web_sys::MediaStreamConstraints::new();
                constraints.video(&wasm_bindgen::JsValue::TRUE);
                let stream: web_sys::MediaStream = wasm_bindgen_futures::JsFuture::from(
                    media_devices
                        .get_user_media_with_constraints(&constraints)
                        .unwrap(),
                )
                .await
                .unwrap()
                .dyn_into()
                .unwrap();

                let scan_barcode = {
                    //let canvas = canvas.clone();
                    move |video: &web_sys::HtmlVideoElement| -> Option<rxing::RXingResult> {
                        if video.video_width() == 0 {
                            return None;
                        }
                        let canvas = web_sys::OffscreenCanvas::new(
                            video.video_width(),
                            video.video_height(),
                        )
                        .unwrap();
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
                        let mut image = BinaryBitmap::new(HybridBinarizer::new(
                            BufferedImageLuminanceSource::new(image::DynamicImage::from(
                                image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                                    canvas.width() as _,
                                    canvas.height() as _,
                                    data,
                                )
                                .unwrap(),
                            )),
                        ));

                        let mut reader = rxing::qrcode::QRCodeReader;

                        reader.decode(&mut image).ok()
                    }
                };

                let video: web_sys::HtmlVideoElement =
                    document.get_element_by_id(&id).unwrap().dyn_into().unwrap();

                video.set_src_object(Some(&stream));

                let fnc = Closure::<dyn FnMut()>::new(move || {
                    if let Some(handler) = &handle_file {
                        if let Some(res) = scan_barcode(&video) {
                            (handler.inner.borrow_mut())(res);
                        }
                    }
                    request_animation_frame2(f.borrow().as_ref().unwrap());
                });
                *g.borrow_mut() = Some(fnc);

                request_animation_frame(g.borrow().as_ref().unwrap());
            }
        }
    });
    render! {
        video {
            id: "{id}",
            autoplay: true,
        },
    }
}
