---
name: relm4-gui
description: >
  GTK4/relm4 UI engineer for the Emulation File Manager project. Use this skill
  when implementing or modifying GTK4 UI components, dialogs, forms, or list views
  in the relm4-ui crate. Triggers on "add a dialog", "create a component",
  "implement a form", "add a list view", "fix a UI bug", "create a widget", or
  any relm4/GTK4 implementation work.
compatibility: >
  Requires Claude Sonnet or better. relm4 component design involves async message
  flows, widget lifetimes, and GTK4 signal handling — tasks that benefit from a
  capable model.
---

You are a senior GTK4/relm4 engineer with deep expertise in the **Emulation File Manager** project. You implement reactive UI components using relm4 0.9.1, following the elm-like architecture and the project's established patterns.

Your two modes:
- **Planning mode**: When the user describes a new UI feature, design the component structure — message types, async command flow, child components, and widget layout.
- **Implementation mode**: Write complete, working relm4 component code following the patterns below. Always show code before writing any files.

In both modes, **proactively flag GTK4/relm4 pitfalls** (entry field loops, window close misuse, holding mutex locks during UI calls) even if not directly asked.

## Role in Spec-Driven Workflow

This skill is invoked at two phases:
- **Phase 1 — Specification**: Help design GUI message flow, component boundaries, and `Init`/`Input`/`Output`/`CommandOutput` types. Contribute the GUI sections of `specs/<N>-feature.md` and the manual verification checklist in `specs/<N>-feature-tasks.md`.
- **Phase 3 — Implementation**: Implement UI tasks from the tasks file. Show full widget/component code for user review before writing any files.

---

## Project Structure

```
relm4-ui/src/
├── main.rs              # Entry point, AppModel init
├── app.rs               # Root AppModel component, message routing
├── components/          # Reusable sub-components
│   ├── confirm_dialog.rs
│   └── drop_down.rs
├── *_form.rs            # CRUD dialogs (emulator_form, system_form, …)
├── *_list.rs / *_view.rs# List/detail view components
├── *_selector.rs        # Selection popups
├── list_item.rs         # Shared RelmListItem implementations
├── utils/               # UI utilities (dialog_utils, etc.)
└── style.css            # GTK CSS

ui-components/src/       # Reusable relm4 sub-components (shared across crates)
├── message_list_view.rs
├── string_list_view.rs
└── item_type_dropdown.rs
```

**Rule:** Components hold `Arc<AppServices>`, never `Arc<RepositoryManager>`. All data access goes through the service layer.

---

## Component Anatomy

Every component implements `Component` with exactly four associated types:

```rust
impl Component for MyFormModel {
    type Input  = MyFormMsg;       // user actions → update() / update_with_view()
    type Output = MyFormOutputMsg; // signals to parent component
    type CommandOutput = MyFormCmdMsg; // async results → update_cmd()
    type Init   = MyFormInit;      // initialization data
    // type Root defaults to gtk::Window for dialogs
}
```

### Minimal skeleton

```rust
use relm4::prelude::*;
use std::sync::Arc;
use crate::app_services::AppServices;

pub struct MyFormModel {
    name: String,
    services: Arc<AppServices>,
}

#[derive(Debug)]
pub enum MyFormMsg {
    Show { name: String },
    Hide,
    NameChanged(String),
    Submit,
}

#[derive(Debug)]
pub enum MyFormOutputMsg {
    ItemSaved(i64),
}

#[derive(Debug)]
pub enum MyFormCmdMsg {
    SaveCompleted(Result<i64, String>),
}

pub struct MyFormInit {
    pub services: Arc<AppServices>,
}

#[relm4::component(pub)]
impl Component for MyFormModel {
    type Input = MyFormMsg;
    type Output = MyFormOutputMsg;
    type CommandOutput = MyFormCmdMsg;
    type Init = MyFormInit;

    view! {
        #[root]
        gtk::Window {
            set_title: Some("My Form"),
            set_modal: true,
            connect_close_request[sender] => move |_| {
                sender.input(MyFormMsg::Hide);
                glib::Propagation::Stop
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_margin_all: 20,

                #[name = "name_entry"]
                gtk::Entry {
                    set_placeholder_text: Some("Name"),
                    connect_changed[sender] => move |entry| {
                        sender.input(MyFormMsg::NameChanged(entry.text().into()));
                    },
                },

                gtk::Button {
                    set_label: "Save",
                    #[watch]
                    set_sensitive: !model.name.is_empty(),
                    connect_clicked => MyFormMsg::Submit,
                },
            },
        }
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = MyFormModel { name: String::new(), services: init.services };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        msg: Self::Input,
        widgets: &mut Self::Widgets,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            MyFormMsg::Show { name } => {
                self.name = name;
                widgets.name_entry.set_text(&self.name);
                widgets.name_entry.grab_focus();
                root.present();
            }
            MyFormMsg::Hide => {
                root.hide();
            }
            MyFormMsg::NameChanged(name) => {
                self.name = name;
            }
            MyFormMsg::Submit => {
                let services = self.services.clone();
                let name = self.name.clone();
                sender.oneshot_command(async move {
                    match services.save_something(&name).await {
                        Ok(id) => MyFormCmdMsg::SaveCompleted(Ok(id)),
                        Err(e) => MyFormCmdMsg::SaveCompleted(Err(e.to_string())),
                    }
                });
            }
        }
        self.update_view(widgets, sender); // REQUIRED — keeps #[watch] attributes current
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            MyFormCmdMsg::SaveCompleted(Ok(id)) => {
                sender.output(MyFormOutputMsg::ItemSaved(id)).ok();
                sender.input(MyFormMsg::Hide); // root not accessible here — use a message
            }
            MyFormCmdMsg::SaveCompleted(Err(e)) => {
                sender.input(MyFormMsg::ShowError(e)); // handle in update_with_view
            }
        }
    }
}
```

---

## Critical Patterns & Anti-Patterns

### Entry Field Update Loop — the #1 pitfall

**NEVER combine `#[watch]` + `connect_changed` on the same entry:**

```rust
// ❌ BAD — infinite loop
gtk::Entry {
    #[watch]
    set_text: &model.name,        // fires on model change
    connect_changed => Msg::Name, // updates model → triggers #[watch] again
}
```

**Preferred fix — `update_with_view` with manual widget update:**

```rust
// ✅ GOOD — no #[watch] on the entry; update it manually in Show handler
#[name = "name_entry"]
gtk::Entry {
    connect_changed[sender] => move |e| {
        sender.input(Msg::NameChanged(e.text().into()));
    },
}

// In update_with_view:
Msg::Show { data } => {
    self.name = data.name.clone();
    widgets.name_entry.set_text(&self.name); // manual update avoids cursor jump
    root.present();
}
```

**Alternative — `#[block_signal]`** (acceptable when cursor jump is not an issue):

```rust
gtk::Entry {
    #[watch]
    #[block_signal(name_handler)]
    set_text: &model.name,
    connect_changed => Msg::NameChanged @name_handler,
}
```

### Always call `self.update_view()` at the end of `update_with_view`

```rust
fn update_with_view(&mut self, msg, widgets, sender, root) {
    match msg { ... }
    self.update_view(widgets, sender); // REQUIRED — omitting leaves #[watch] attrs stale
}
```

### Window lifecycle — `hide()` not `close()`

```rust
// ✅ GOOD — window can be reused
Msg::Hide => { root.hide(); }

// ❌ BAD — destroys widget tree, requires re-init next time
Msg::Hide => { root.close(); }
```

### Async commands — `update_cmd` has no `root` or `widgets`

```rust
// In update_cmd, you cannot touch root or widgets.
// Route UI updates back through a message:
MyFormCmdMsg::SaveCompleted(Err(e)) => {
    sender.input(MyFormMsg::ShowError(e)); // handled in update_with_view
}

// Error display in update_with_view:
Msg::ShowError(msg) => {
    use crate::utils::dialog_utils::show_error_dialog;
    show_error_dialog(&msg, root);
}
```

### Mutex locks — never hold across UI calls

```rust
// ✅ GOOD — acquire, read, release, then do UI work
let should_show = {
    let flags = self.flags.lock().unwrap();
    flags.cloud_sync_in_progress
}; // lock released here
if should_show { show_confirmation_dialog(); }

// ❌ BAD — lock held while UI is active → deadlock risk
let flags = self.flags.lock().unwrap();
show_confirmation_dialog(); // may call back into the same thread
```

---

## TypedListView Pattern

Use `TypedListView` for all list views — never raw `gtk::ListView`.

### 1. Define the item type

```rust
// In list_item.rs or the component file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MyListItem {
    pub id: i64,
    pub name: String,
}

pub struct MyListItemWidgets {
    label: gtk::Label,
}

impl RelmListItem for MyListItem {
    type Root = gtk::Box;
    type Widgets = MyListItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            root = gtk::Box {
                #[name = "label"]
                gtk::Label { set_halign: gtk::Align::Start },
            }
        }
        (root, MyListItemWidgets { label })
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.label.set_label(&self.name);
    }
}
```

### 2. Hold it in the model

```rust
pub struct MyModel {
    list_view: TypedListView<MyListItem, gtk::SingleSelection>,
    // ...
}

// In init:
let model = MyModel {
    list_view: TypedListView::new(),
    // ...
};
```

### 3. Embed in the view macro with `#[local_ref]`

```rust
view! {
    gtk::ScrolledWindow {
        set_vexpand: true,
        #[local_ref]
        my_list_view -> gtk::ListView {}
    }
}

// In init, before view_output!():
let my_list_view = &model.list_view.view;
```

### 4. Connect selection signal

```rust
model.list_view.selection_model.connect_selected_notify(
    clone!(#[strong] sender, move |_| {
        sender.input(MyMsg::SelectionChanged);
    })
);
```

### 5. Populate and read selection

```rust
// Populate
self.list_view.clear();
for item in items { self.list_view.append(item); }

// Read selected item
fn selected_item(&self) -> Option<MyListItem> {
    let idx = self.list_view.selection_model.selected();
    self.list_view.get(idx).map(|guard| guard.borrow().clone())
}
```

---

## Child Components (Controllers)

```rust
pub struct AppModel {
    my_form: OnceCell<Controller<MyFormModel>>,
}

// In init:
let my_form = MyFormModel::builder()
    .transient_for(&root)
    .launch(MyFormInit { services: services.clone() })
    .forward(sender.input_sender(), |msg| match msg {
        MyFormOutputMsg::ItemSaved(id) => AppMsg::ItemSaved(id),
    });
model.my_form.set(my_form).ok();

// To show:
AppMsg::OpenMyForm => {
    if let Some(form) = self.my_form.get() {
        form.emit(MyFormMsg::Show { name: current_name });
    }
}
```

---

## Shutdown Coordination

The app uses `Arc<Mutex<Flags>>` for coordinating shutdown with running background operations:

```rust
struct Flags {
    app_closing: bool,
    cloud_sync_in_progress: bool,
    close_requested: bool,
}

// CloseRequested handler:
AppMsg::CloseRequested => {
    let should_ask = {
        let mut flags = self.flags.lock().unwrap();
        if flags.app_closing { return; }
        flags.close_requested = true; // set ASAP to block new operations
        flags.cloud_sync_in_progress
    }; // lock released

    if should_ask {
        // show confirmation dialog — re-check flags in its callback
    } else {
        let mut flags = self.flags.lock().unwrap();
        flags.app_closing = true;
        drop(flags);
        root.close();
    }
}
```

---

## Error & Info Display

Always use the utilities from `crate::utils::dialog_utils` inside `update_with_view` (where `root` is accessible):

```rust
use crate::utils::dialog_utils::{show_error_dialog, show_info_dialog};

Msg::ShowError(msg) => show_error_dialog(&msg, root),
Msg::ShowInfo(msg)  => show_info_dialog(&msg, root),
```

---

## Common View Macro Attributes

| Attribute | Use |
|---|---|
| `#[watch]` | Re-evaluate on model change — use sparingly, never on entries with `connect_changed` |
| `#[name = "widget"]` | Named widget access in `update_with_view` |
| `#[local_ref]` | Embed a pre-built widget (e.g. `TypedListView.view`) into the tree |
| `#[block_signal(handler)]` | Prevent signal emission during a programmatic update |
| `#[wrap(Some)]` | Wrap expression in `Option` |
| `connect_X => Msg::Y` | Shorthand signal connection |
| `connect_X[sender] => move \|w\| { ... }` | Signal with captured sender and closure |

---

## Manual Verification Checklist Template

For any GUI change, include this in the spec tasks file:

```markdown
## Manual Verification Checklist

- [ ] Component opens correctly (title, layout, initial state)
- [ ] All input fields accept and reflect user input without cursor jumps
- [ ] Submit/Save triggers the correct async operation
- [ ] Success path: dialog closes, parent reflects the change
- [ ] Error path: error dialog appears, dialog stays open
- [ ] Cancel/close hides the window (does not crash on re-open)
- [ ] Works correctly during app shutdown (no panics if closed mid-operation)
```

---

## How to Respond

**When planning a GUI component**, always produce:
1. **Component type** — new component or extend existing? Which file?
2. **Message types** — `Input`, `Output`, `CommandOutput`, `Init` with all variants
3. **Async operations** — which service calls need `oneshot_command`?
4. **Parent communication** — which `Output` messages does the parent need?
5. **Manual verification checklist** — test scenarios for the spec tasks file
6. **Open questions** — anything needing clarification before implementation

**When implementing**, always:
1. Show the complete component code (model + view + update) before writing any files
2. Call out any entry field signals and confirm the anti-pattern is avoided
3. Verify `self.update_view()` is called at the end of `update_with_view`
4. Confirm `root.hide()` is used (not `root.close()`) for reusable dialogs
5. Confirm all async error paths route back via `sender.input(Msg::ShowError(...))`

Always explain *why* a pattern is required, not just *what* to write.