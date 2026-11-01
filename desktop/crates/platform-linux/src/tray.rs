//! Tray icon (StatusNotifierItem). Clicking it shows or hides the window; the
//! icon itself shows the battery the way the App page configures it.

use droidpods_core::TrayBattery;
use ksni::{menu::StandardItem, Icon, MenuItem, ToolTip, TrayMethods};

/// Earbuds glyph from the design (`TrayIcons`), 24×24 viewBox.
const PODS: &str = "M8 3.5C5 3.5 3 5.6 3 8.4c0 2.3 1.4 4 3.7 4.4.9.2 1.3.6 1.3 1.5v5.2a1.7 1.7 0 0 0 3.4 0V7.2C11.4 4.9 10.3 3.5 8 3.5ZM16 3.5c3 0 5 2.1 5 4.9 0 2.3-1.4 4-3.7 4.4-.9.2-1.3.6-1.3 1.5v5.2a1.7 1.7 0 0 1-3.4 0V7.2c0-2.3 1.1-3.7 3.4-3.7Z";

/// What the icon shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayView {
    pub connected: bool,
    /// Lowest bud (or headset) level.
    pub level: Option<u8>,
    pub style: TrayBattery,
    /// Light panel: draw dark.
    pub light: bool,
    pub tooltip: String,
}

pub struct Tray {
    view: TrayView,
    on_activate: Box<dyn Fn() + Send>,
    on_quit: Box<dyn Fn() + Send>,
}

pub type TrayHandle = ksni::Handle<Tray>;

impl Tray {
    pub async fn spawn(
        view: TrayView,
        on_activate: impl Fn() + Send + 'static,
        on_quit: impl Fn() + Send + 'static,
    ) -> Result<TrayHandle, ksni::Error> {
        Tray { view, on_activate: Box::new(on_activate), on_quit: Box::new(on_quit) }.spawn().await
    }

    pub fn set_view(&mut self, view: TrayView) {
        self.view = view;
    }
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "droidpods".into()
    }

    fn title(&self) -> String {
        "DroidPods".into()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        (self.on_activate)();
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        [22, 32, 48, 64].into_iter().filter_map(|size| render(&self.view, size)).collect()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip { title: "DroidPods".into(), description: self.view.tooltip.clone(), ..Default::default() }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Öffnen".into(),
                activate: Box::new(|t: &mut Self| (t.on_activate)()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Beenden".into(),
                activate: Box::new(|t: &mut Self| (t.on_quit)()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn svg(view: &TrayView) -> String {
    let fg = if view.light { "#12161D" } else { "#EEF1F6" };
    let body = match (view.connected, view.level, view.style) {
        (false, _, _) => {
            format!(r#"<g opacity=".45"><path d="{PODS}"/><path d="M3 21 21 3" stroke="{fg}" stroke-width="2"/></g>"#)
        }
        (true, Some(l), TrayBattery::Ring) => {
            // resvg ignores `pathLength`, so dash in real units.
            let circumference = 2.0 * std::f32::consts::PI * 10.5;
            let arc = circumference * l.min(100) as f32 / 100.0;
            format!(
                r#"<circle cx="12" cy="12" r="10.5" fill="none" stroke="{fg}" stroke-opacity=".25" stroke-width="1.8"/>
                   <circle cx="12" cy="12" r="10.5" fill="none" stroke="{fg}" stroke-width="1.8" stroke-linecap="round"
                     stroke-dasharray="{arc} {circumference}" transform="rotate(-90 12 12)"/>
                   <path d="{PODS}" transform="translate(6 6) scale(.5)"/>"#
            )
        }
        (true, Some(l), TrayBattery::Percent) => digits(l.min(99)),
        _ => format!(r#"<path d="{PODS}"/>"#),
    };
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="{fg}">{body}</svg>"#)
}

/// Two digits as rounded seven-segment glyphs, so no font is needed.
fn digits(n: u8) -> String {
    // Segments a–g of a 7.5×14 cell.
    const SEG: [(f32, f32, f32, f32); 7] = [
        (1.0, 0.0, 5.5, 2.0),  // a
        (5.5, 1.0, 2.0, 6.0),  // b
        (5.5, 7.0, 2.0, 6.0),  // c
        (1.0, 12.0, 5.5, 2.0), // d
        (0.0, 7.0, 2.0, 6.0),  // e
        (0.0, 1.0, 2.0, 6.0),  // f
        (1.0, 6.0, 5.5, 2.0),  // g
    ];
    const ON: [u8; 10] = [0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F];
    let text = n.to_string();
    let width = text.len() as f32 * 9.5 - 2.0;
    let mut out = String::new();
    for (i, ch) in text.bytes().enumerate() {
        let x0 = (24.0 - width) / 2.0 + i as f32 * 9.5;
        let mask = ON[(ch - b'0') as usize];
        for (s, (x, y, w, h)) in SEG.iter().enumerate() {
            if mask & (1 << s) != 0 {
                out += &format!(r#"<rect x="{}" y="{}" width="{w}" height="{h}" rx="1"/>"#, x0 + x, 5.0 + y);
            }
        }
    }
    out
}

fn render(view: &TrayView, size: u32) -> Option<Icon> {
    let tree = resvg::usvg::Tree::from_str(&svg(view), &Default::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
    let scale = size as f32 / 24.0;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    // tiny-skia gives premultiplied RGBA; SNI wants ARGB32 in network byte order.
    let data = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.alpha(), c.red(), c.green(), c.blue()]
        })
        .collect();
    Some(Icon { width: size as i32, height: size as i32, data })
}
