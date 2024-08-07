use crate::backend::config::Farm;
use crate::frontend::configuration::MaybeValid;
use bytesize::ByteSize;
use gtk::prelude::*;
// TODO: Remove import once in prelude: https://github.com/Relm4/Relm4/issues/662
use relm4::factory::AsyncFactoryComponent;
use relm4::prelude::*;
// TODO: Remove import once in prelude: https://github.com/Relm4/Relm4/issues/662
use crate::frontend::configuration::utils::is_directory_writable;
use crate::frontend::translations::{AsDefaultStr, T};
use crate::icon_names;
use relm4::AsyncFactorySender;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::warn;

// 2 GB
const MIN_FARM_SIZE: u64 = 1000 * 1000 * 1000 * 2;

fn is_size_valid(size: &str) -> bool {
    ByteSize::from_str(size)
        .map(|size| size.as_u64() >= MIN_FARM_SIZE)
        .unwrap_or_default()
}

#[derive(Debug)]
pub(super) struct FarmWidgetInit {
    pub(super) path: PathBuf,
    pub(super) size: String,
}

impl Default for FarmWidgetInit {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            size: String::new(),
        }
    }
}

#[derive(Debug)]
pub(super) enum FarmWidgetInput {
    DirectorySelected(PathBuf),
    FarmSizeChanged(String),
}

#[derive(Debug)]
pub(super) enum FarmWidgetOutput {
    OpenDirectory(DynamicIndex),
    ValidityUpdate,
    Delete(DynamicIndex),
}

#[derive(Debug)]
pub(super) struct FarmWidget {
    // TODO: Track changes for dynamic index
    index: DynamicIndex,
    path: MaybeValid<PathBuf>,
    size: MaybeValid<String>,
}

#[relm4::factory(pub(super) async)]
impl AsyncFactoryComponent for FarmWidget {
    type Init = FarmWidgetInit;
    type Input = FarmWidgetInput;
    type Output = FarmWidgetOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::ListBoxRow {
            set_activatable: false,
            set_margin_bottom: 10,
            set_selectable: false,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,

                gtk::Label {
                    add_css_class: "heading",
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: T.configuration_farm(self.index.current_index()).as_str(),
                },

                gtk::Box {
                    set_spacing: 10,

                    gtk::Box {
                        add_css_class: "linked",

                        gtk::Entry {
                            #[track = "self.path.changed_is_valid()"]
                            set_css_classes: if self.path.is_valid {
                                &["valid-input"]
                            } else {
                                &["invalid-input"]
                            },
                            set_can_focus: false,
                            set_editable: false,
                            set_hexpand: true,
                            set_placeholder_text: Some(
                                T
                                    .configuration_farm_path_placeholder(
                                        if cfg!(windows) {
                                            "D:\\subspace-farm"
                                        } else if cfg!(target_os = "macos") {
                                            "/Volumes/Subspace/subspace-farm"
                                        } else {
                                            "/media/subspace-farm"
                                        },
                                    )
                                    .as_str(),
                            ),
                            set_primary_icon_name: Some(icon_names::SSD),
                            set_primary_icon_activatable: false,
                            set_primary_icon_sensitive: false,
                            #[track = "self.path.changed_is_valid()"]
                            set_secondary_icon_name: self.path.icon(),
                            set_secondary_icon_activatable: false,
                            set_secondary_icon_sensitive: false,
                            #[track = "self.path.changed_value()"]
                            set_text: self.path.display().to_string().as_str(),
                            set_tooltip_markup: Some(
                                &T.configuration_farm_path_tooltip()
                            ),
                        },

                        gtk::Button {
                            connect_clicked[sender, index] => move |_| {
                                if sender.output(FarmWidgetOutput::OpenDirectory(index.clone())).is_err() {
                                    warn!("Can't send open directory output");
                                }
                            },
                            set_label: &T.configuration_farm_path_button_select(),
                        },
                    },

                    gtk::Entry {
                        connect_activate[sender] => move |entry| {
                            sender.input(FarmWidgetInput::FarmSizeChanged(entry.text().into()));
                        },
                        connect_changed[sender] => move |entry| {
                            sender.input(FarmWidgetInput::FarmSizeChanged(entry.text().into()));
                        },
                        #[track = "self.size.changed_is_valid()"]
                        set_css_classes: if self.size.is_valid {
                            &["valid-input"]
                        } else {
                            &["invalid-input"]
                        },
                        set_placeholder_text: Some(
                            &T.configuration_farm_size_placeholder(),
                        ),
                        set_primary_icon_name: Some(icon_names::SIZE_HORIZONTALLY),
                        set_primary_icon_activatable: false,
                        set_primary_icon_sensitive: false,
                        #[track = "self.size.changed_is_valid()"]
                        set_secondary_icon_name: self.size.icon(),
                        set_secondary_icon_activatable: false,
                        set_secondary_icon_sensitive: false,
                        #[track = "self.size.changed_value()"]
                        set_text: self.size.as_str(),
                        set_tooltip_markup: Some(
                            &T.configuration_farm_size_tooltip()
                        ),
                    },

                    gtk::Button {
                        connect_clicked[sender, index] => move |_| {
                            if sender.output(FarmWidgetOutput::Delete(index.clone())).is_err() {
                                warn!("Can't send delete output");
                            }
                        },
                        set_icon_name: icon_names::CROSS,
                        set_tooltip: &T.configuration_farm_delete(),
                    },
                },

                gtk::Label {
                    add_css_class: "error-label",
                    set_halign: gtk::Align::Start,
                    set_label: &T.configuration_farm_path_error_doesnt_exist_or_write_permissions(),
                    #[track = "self.path.changed_is_valid()"]
                    set_visible: !self.path.is_valid && self.path.value != PathBuf::new(),
                },
            },
        }
    }

    async fn init_model(
        value: Self::Init,
        index: &DynamicIndex,
        sender: AsyncFactorySender<Self>,
    ) -> Self {
        let instance = Self {
            index: index.clone(),
            path: if is_directory_writable(value.path.clone()).await {
                MaybeValid::yes(value.path)
            } else {
                MaybeValid::no(value.path)
            },
            size: if is_size_valid(&value.size) {
                MaybeValid::yes(value.size)
            } else {
                MaybeValid::no(value.size)
            },
        };

        // Send notification up that validity was updated, such that parent view can re-render
        // view if necessary, this is necessary due to async initialization of the model
        if sender.output(FarmWidgetOutput::ValidityUpdate).is_err() {
            warn!("Can't send validity update output");
        }

        instance
    }

    async fn update(&mut self, input: Self::Input, sender: AsyncFactorySender<Self>) {
        // Reset changes
        self.path.reset();
        self.size.reset();

        let was_valid = self.valid();

        match input {
            FarmWidgetInput::DirectorySelected(path) => {
                self.path = if is_directory_writable(path.clone()).await {
                    MaybeValid::yes(path)
                } else {
                    MaybeValid::no(path)
                };
            }
            FarmWidgetInput::FarmSizeChanged(size) => {
                self.size.set_is_valid(is_size_valid(&size));
                self.size.value = size;
            }
        }

        let is_valid = self.valid();
        if was_valid != is_valid {
            // Send notification up that validity was updated, such that parent view can re-render
            // view if necessary
            if sender.output(FarmWidgetOutput::ValidityUpdate).is_err() {
                warn!("Can't send validity update output");
            }
        }
    }
}

impl FarmWidget {
    pub(super) fn valid(&self) -> bool {
        self.path.is_valid && self.size.is_valid
    }

    pub(super) fn farm(&self) -> Farm {
        Farm {
            path: PathBuf::clone(&self.path),
            size: String::clone(&self.size),
        }
    }
}
