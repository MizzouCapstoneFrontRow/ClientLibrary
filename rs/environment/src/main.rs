use gio::{traits::ApplicationExt, prelude::ApplicationExtManual};
use gtk::{Window, Application, traits::{GtkApplicationExt, GtkWindowExt}, Builder, prelude::BuilderExtManual};

fn main() {
    // Register and include resources
    gio::resources_register_include!("compiled.gresource")
        .expect("Failed to register resources.");

    // Create a new application
    let app = Application::builder()
        .application_id("com.github.mizzoucapstonefrontrow.environment")
        .build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run();
}

fn build_ui(app: &Application) {
    let builder = Builder::from_resource("/mizzoucapstonefrontrow/environment/environment.ui");
    let connect_window: Window = builder.object("connect_dialog").unwrap();
    app.add_window(&connect_window);
    connect_window.present();
}
