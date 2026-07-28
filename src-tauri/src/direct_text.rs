#![cfg(windows)]

use windows::{
    core::w,
    Win32::{
        Foundation::RECT,
        Graphics::{
            Direct2D::{
                Common::{D2D1_ALPHA_MODE_IGNORE, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F},
                D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
                D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_FACTORY_TYPE_SINGLE_THREADED,
                D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
                D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
                D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
            },
            DirectWrite::{
                DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED,
                DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT,
                DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING,
                DWRITE_WORD_WRAPPING_NO_WRAP,
            },
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::HDC,
        },
    },
};

pub struct DirectTextCanvas {
    target: ID2D1DCRenderTarget,
    factory: IDWriteFactory,
}

impl DirectTextCanvas {
    pub unsafe fn begin(
        hdc: *mut core::ffi::c_void,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> windows::core::Result<Self> {
        let d2d_factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_IGNORE,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let target = unsafe { d2d_factory.CreateDCRenderTarget(&properties)? };
        let bounds = RECT {
            left,
            top,
            right,
            bottom,
        };
        unsafe {
            target.BindDC(HDC(hdc), &bounds)?;
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            target.BeginDraw();
        }
        let factory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        Ok(Self { target, factory })
    }

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn draw(
        &self,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: u32,
        font_size: i32,
        font_weight: i32,
    ) -> windows::core::Result<()> {
        let weight = DWRITE_FONT_WEIGHT(font_weight.clamp(1, 999));
        let format = unsafe {
            self.factory.CreateTextFormat(
                w!("Malgun Gothic"),
                None,
                if font_weight <= 0 {
                    DWRITE_FONT_WEIGHT_NORMAL
                } else {
                    weight
                },
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_size.max(1) as f32,
                w!("ko-KR"),
            )?
        };
        unsafe {
            format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING)?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
        }

        let color = color_from_colorref(color);
        let brush = unsafe { self.target.CreateSolidColorBrush(&color, None)? };
        let bounds = D2D_RECT_F {
            left: x as f32,
            top: y as f32,
            right: (x + width) as f32,
            bottom: (y + height) as f32,
        };
        let text = text.encode_utf16().collect::<Vec<_>>();
        unsafe {
            self.target.DrawText(
                &text,
                &format,
                &bounds,
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
        Ok(())
    }

    pub unsafe fn finish(&self) -> windows::core::Result<()> {
        unsafe { self.target.EndDraw(None, None) }
    }
}

fn color_from_colorref(color: u32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: (color & 0xff) as f32 / 255.0,
        g: ((color >> 8) & 0xff) as f32 / 255.0,
        b: ((color >> 16) & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}
