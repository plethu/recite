use gtk::{glib, prelude::*};

fn main() -> glib::ExitCode {
    let application = gtk::Application::builder()
        .application_id("org.recite.Bakeoff")
        .build();
    application.connect_activate(recite_bakeoff_gtk::build);
    application.run()
}
