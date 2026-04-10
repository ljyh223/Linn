//! Main component of the application.

use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::AdwApplicationWindowExt;
use relm4::gtk::glib;
use relm4::gtk::prelude::GtkWindowExt;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent, adw};

use crate::ui::header::Header;

relm4::new_action_group!(pub WindowActionGroup, "win");
relm4::new_stateless_action!(pub CloseAction, WindowActionGroup, "close");

pub struct Window {
    /// Header bar component containing the full layout.
    pub header: Controller<Header>,
}

#[relm4::component(pub)]
impl SimpleComponent for Window {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        #[root]
        adw::ApplicationWindow {
            set_default_height: 700,
            set_default_width: 850,
            set_content: Some(model.header.widget()),
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let app = relm4::main_adw_application();
        app.set_accelerators_for_action::<CloseAction>(&["<Ctrl>W"]);

        let mut action_group = RelmActionGroup::<WindowActionGroup>::new();

        let close_action = RelmAction::<CloseAction>::new_stateless(glib::clone!(
            #[weak]
            root,
            move |_| {
                root.close();
            }
        ));

        action_group.add_action(close_action);
        action_group.register_for_widget(&root);

        let header = Header::builder().launch(()).detach();

        let model = Self { header };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
