#![allow(non_snake_case)]

use dioxus_router::prelude::*;

use dioxus::prelude::*;
use log::LevelFilter;

fn main() {
    // Init debug
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");
    console_error_panic_hook::set_once();

    log::info!("starting app");
    dioxus_web::launch(app);
}

fn app(cx: Scope) -> Element {
    render! {
        Router::<Route> {}
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

#[inline_props]
fn Blog(cx: Scope, id: i32) -> Element {
    render! {
        Link { to: Route::Home {}, "Go to counter" }
        img {
            src: "a.png",
        }
        "Blog post {id}"
    }
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    use wasm_bindgen::JsCast;

    let mut count = use_state(cx, || 0);
    let state = use_state(cx, || ());
    use_future(cx, (count,), |_| async move {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let img: web_sys::HtmlImageElement = document
            .get_element_by_id("code")
            .unwrap()
            .dyn_into()
            .unwrap();
        let canvas: web_sys::HtmlCanvasElement = document
            .create_element("canvas")
            .unwrap()
            .dyn_into()
            .unwrap();
        let ctx: web_sys::CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();
        canvas.set_width(img.width());
        canvas.set_height(img.height());
        ctx.draw_image_with_html_image_element_and_dw_and_dh(
            &img,
            0.0,
            0.0,
            img.width() as _,
            img.height() as _,
        );
        use std::ops::Deref;
        let data: Vec<_> = ctx
            .get_image_data(0.0, 0.0, img.width() as _, img.height() as _)
            .unwrap()
            .data()
            .clone()
            .deref()
            .to_vec();

        let video: web_sys::HtmlVideoElement = document
            .query_selector("video")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();
        let media_devices = window.navigator().media_devices().unwrap();
        let mut cons = web_sys::MediaStreamConstraints::new();
        cons.video(&wasm_bindgen::JsValue::TRUE);
        let stream: web_sys::MediaStream = wasm_bindgen_futures::JsFuture::from(
            media_devices
                .get_user_media_with_constraints(&cons)
                .unwrap(),
        )
        .await
        .unwrap()
        .dyn_into()
        .unwrap();
        video.set_src_object(Some(&stream));

        let request_animation_frame = {
            let window = window.clone();
            move |f: &Closure<dyn FnMut()>| {
                window
                    .request_animation_frame(f.as_ref().unchecked_ref())
                    .expect("should register `requestAnimationFrame` OK");
            }
        };
        let scan_barcode = {
            //let canvas = canvas.clone();
            move |video: &web_sys::HtmlVideoElement| -> Option<String> {
                if video.video_width() == 0 {
                    return None;
                }
                let canvas =
                    web_sys::OffscreenCanvas::new(video.video_width(), video.video_height())
                        .unwrap();
                let ctx: web_sys::OffscreenCanvasRenderingContext2d = canvas
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into()
                    .unwrap();
                /*if canvas.width() != video.video_width() {
                    canvas.set_width(video.video_width());
                }
                if canvas.height() != video.video_height() {
                    canvas.set_height(video.video_height());
                }*/

                ctx.draw_image_with_html_video_element_and_dw_and_dh(
                    &video,
                    0.0,
                    0.0,
                    canvas.width() as _,
                    canvas.height() as _,
                );

                ctx.get_image_data(0.0, 0.0, canvas.width() as _, canvas.height() as _)
                    .unwrap();
                if let Ok(data) =
                    ctx.get_image_data(0.0, 0.0, canvas.width() as _, canvas.height() as _)
                {
                    //log::info!("{} {} {} {}", 0.0, 0.0, canvas.width(), canvas.height());
                    let data = data.data().clone().deref().to_vec();

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

                    let res = reader.decode(&mut image);
                    match res {
                        Ok(v) => {
                            log::info!("{v}")
                        }
                        Err(e) => log::error!("{e}"),
                    }
                }

                None
            }
        };
        use std::rc::Rc;
        use wasm_bindgen::prelude::*;
        // TODO: should probably be requestVideoFrameCallback
        let f = Rc::new(RefCell::new(None));
        let request_animation_frame2 = request_animation_frame.clone();
        let g = f.clone();
        *g.borrow_mut() = Some(Closure::<dyn FnMut() -> ()>::new(move || {
            log::info!("{:?}", scan_barcode(&video));
            //window.request_animation_frame(f.borrow().as_ref().unwrap());
            request_animation_frame2(f.borrow().as_ref().unwrap());
        }));
        request_animation_frame(g.borrow().as_ref().unwrap());

        /*//let buf: image::ImageBuffer<image::Rgba<_>, Vec<u8>> =
        //image::ImageBuffer::from_raw(img.width(), img.height(), data).unwrap();
        let (width, height) = (img.width(), img.height());
        //let img: image::DynamicImage = image::DynamicImage::from(buf);
        use rxing::common::BitMatrix;

        pub fn convert_js_image_to_luma(data: &[u8]) -> Vec<u32> {
            let mut luma_data = Vec::new();
            for src_pixel in data.chunks_exact(4) {
                let [red, green, blue, alpha] = src_pixel else {
                    continue;
                };
                let pixel = if *alpha == 0 {
                    // white, so we know its luminance is 255
                    0xFF
                } else {
                    // .299R + 0.587G + 0.114B (YUV/YIQ for PAL and NTSC),
                    // (306*R) >> 10 is approximately equal to R*0.299, and so on.
                    // 0x200 >> 10 is 0.5, it implements rounding.

                    ((306 * (*red as u64) + 601 * (*green as u64) + 117 * (*blue as u64) + 0x200)
                        >> 10) as u32
                };
                luma_data.push(pixel);
            }

            luma_data
        }
        //let bm = BitMatrix::try_from(img).unwrap();
        use rxing::qrcode::QRCodeReader;

        let mut multi_format_reader = QRCodeReader::default();
        let data = convert_js_image_to_luma(&data);

        match multi_format_reader.decode(&mut rxing::BinaryBitmap::new(
            rxing::common::HybridBinarizer::new(
                rxing::RGBLuminanceSource::new_with_width_height_pixels(
                    width as usize,
                    height as usize,
                    &data,
                ),
            ),
        )) {
            Ok(_) => log::info!("ok"),
            Err(e) => log::error!("fail {}", e),
        };*/
    });

    cx.render(rsx! {
        Link {
            to: Route::Blog {
                id: *count.get()
            },
            "Go to blog"
        }
        img {
            id: "code",
            width: 220,
            height: 200,
            src: "a.png",
        }
        video {
            autoplay: true,
        }
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }

        }
    })
}
