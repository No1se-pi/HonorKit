use gtk::{cairo, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    Scan,
    Lift,
    Passed,
    Error,
}

pub struct Art {
    pub widget: gtk::DrawingArea,
    mode: Rc<Cell<(Mode, i64)>>,
    tick: RefCell<Option<gtk::TickCallbackId>>,
}

impl Art {
    pub fn new() -> Self {
        let widget = gtk::DrawingArea::builder()
            .content_width(132)
            .content_height(132)
            .halign(gtk::Align::Center)
            .build();
        let mode = Rc::new(Cell::new((Mode::Idle, glib::monotonic_time())));
        let paint_mode = mode.clone();
        widget.set_draw_func(move |_, cr, width, height| {
            let (mode, start) = paint_mode.get();
            let motion = gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations());
            let elapsed = (glib::monotonic_time() - start) as f64 / 1_000_000.0;
            let phase = if motion {
                elapsed.rem_euclid(1.8) / 1.8
            } else {
                0.48
            };
            paint(cr, f64::from(width), f64::from(height), mode, phase);
        });
        Self {
            widget,
            mode,
            tick: RefCell::new(None),
        }
    }

    pub fn set(&self, mode: Mode) {
        if self.mode.get().0 == mode {
            return;
        }
        self.mode.set((mode, glib::monotonic_time()));
        let mut tick = self.tick.borrow_mut();
        if matches!(mode, Mode::Scan | Mode::Lift) {
            if tick.is_none() {
                *tick = Some(self.widget.add_tick_callback(|widget, _| {
                    if gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations()) {
                        widget.queue_draw();
                    }
                    glib::ControlFlow::Continue
                }));
            }
        } else if let Some(callback) = tick.take() {
            callback.remove();
        }
        self.widget.queue_draw();
    }
}

fn paint(cr: &cairo::Context, width: f64, height: f64, mode: Mode, phase: f64) {
    let radius = width.min(height) / 132.0;
    cr.translate(width / 2.0, height / 2.0);
    cr.scale(radius, radius);
    let (red, green, blue) = match mode {
        Mode::Passed => (0.23, 0.78, 0.56),
        Mode::Error => (0.98, 0.42, 0.42),
        Mode::Lift => (0.98, 0.70, 0.39),
        Mode::Idle | Mode::Scan => (0.39, 0.68, 1.0),
    };

    cr.set_source_rgba(red, green, blue, 0.15);
    cr.arc(0.0, 0.0, 57.0, 0.0, std::f64::consts::TAU);
    let _ = cr.fill_preserve();
    cr.set_line_width(1.2);
    cr.set_source_rgba(red, green, blue, 0.32);
    let _ = cr.stroke();
    if matches!(mode, Mode::Scan | Mode::Lift) {
        cr.set_source_rgba(red, green, blue, (1.0 - phase) * 0.40);
        cr.arc(0.0, 0.0, 57.0 + phase * 8.0, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();
    }

    cr.set_line_width(2.6);
    cr.set_line_cap(cairo::LineCap::Round);
    let sweep = phase * 82.0 - 41.0;
    let ink = cairo::LinearGradient::new(0.0, sweep - 16.0, 0.0, sweep + 16.0);
    if mode == Mode::Scan {
        ink.add_color_stop_rgba(0.0, red, green, blue, 0.62);
        ink.add_color_stop_rgba(0.5, 0.85, 0.96, 1.0, 1.0);
        ink.add_color_stop_rgba(1.0, red, green, blue, 0.62);
        let _ = cr.set_source(&ink);
    } else {
        cr.set_source_rgba(red, green, blue, 0.94);
    }

    cr.move_to(-29.0, -22.0);
    cr.curve_to(-15.0, -45.0, 18.0, -45.0, 31.0, -21.0);
    cr.move_to(-35.0, 1.0);
    cr.curve_to(-33.0, -45.0, 33.0, -40.0, 35.0, 3.0);
    cr.move_to(-30.0, 17.0);
    cr.curve_to(-29.0, 6.0, -32.0, -12.0, -15.0, -22.0);
    cr.curve_to(7.0, -35.0, 32.0, -13.0, 28.0, 19.0);
    cr.move_to(-24.0, 26.0);
    cr.curve_to(-19.0, 16.0, -25.0, 2.0, -18.0, -11.0);
    cr.curve_to(-8.0, -29.0, 26.0, -18.0, 21.0, 19.0);
    cr.curve_to(20.0, 24.0, 19.0, 29.0, 17.0, 32.0);
    cr.move_to(-17.0, 33.0);
    cr.curve_to(-11.0, 21.0, -18.0, 12.0, -11.0, -4.0);
    cr.curve_to(-5.0, -16.0, 13.0, -12.0, 14.0, 7.0);
    cr.curve_to(15.0, 19.0, 12.0, 30.0, 8.0, 36.0);
    cr.move_to(-8.0, 36.0);
    cr.curve_to(-3.0, 26.0, -8.0, 15.0, -5.0, 5.0);
    cr.curve_to(-2.0, -4.0, 6.0, -2.0, 7.0, 8.0);
    cr.curve_to(8.0, 18.0, 5.0, 31.0, 1.0, 37.0);
    cr.move_to(-1.0, 9.0);
    cr.curve_to(1.0, 16.0, 0.0, 24.0, -2.5, 30.0);
    let _ = cr.stroke();
}
