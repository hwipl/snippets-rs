use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Button, DropDown, Label, Orientation};

const APP_ID: &str = "com.github.hwipl.snippets-rs.gtk4-rs.hello";

fn build_ui(app: &Application) {
    // Create a button with label
    let button = Button::builder().label("greet!").build();

    // Create a dropdown
    let options = &["hello!", "hi!", "good day!", "greetings!"];
    let dropdown = DropDown::from_strings(options);

    // Create inner box
    let inner_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .build();

    // Add button and dropdown to inner box
    inner_box.append(&dropdown);
    inner_box.append(&button);

    // Create outer box
    let outer_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    // Create label for output
    let label = Label::builder().label("select greeting").build();

    // Add inner box and label to outer box
    outer_box.append(&inner_box);
    outer_box.append(&label);

    // Connect to "clicked" signal of `button`
    button.connect_clicked(move |_button| {
        // Set label to selected greeting
        let s: usize = dropdown.selected().try_into().unwrap();
        label.set_label(options[s]);
    });

    // Create a window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Hello!")
        .child(&outer_box)
        .build();

    // Present window
    window.present();
}

fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}
