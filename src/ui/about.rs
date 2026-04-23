//! About dialog.

use relm4::gtk::License;
use relm4::{ComponentParts, SimpleComponent, adw};

/// About dialog.
pub struct About {}

#[relm4::component(pub)]
impl SimpleComponent for About {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        #[name(dialog)]
        adw::AboutDialog {
            set_application_name: "linn",
            set_developer_name: "ljyh",
            set_version: env!("CARGO_PKG_VERSION"),
            set_developers: &["ljyh"],
            set_copyright: "@2026 ljyh",
            set_license_type: License::Gpl30,
        }
    }

    fn init(_init: Self::Init, root: Self::Root, _sender: relm4::ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self {};
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
