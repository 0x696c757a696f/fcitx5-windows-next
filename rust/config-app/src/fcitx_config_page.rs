#![forbid(unsafe_code)]

//! Settings pages generated from Fcitx's own configuration descriptions, the
//! way fcitx5-macos and fcitx5-android show addon and input method options.
//! Every control saves on its own: a change is sent to the running engine a
//! moment after the last edit, and the page keeps the values before each save
//! so it can undo.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use windui::core::EventCtx;
use windui::prelude::{
    signal as windui_signal, Align as WindUiAlign, App as WindUiApp, Element as WindUiElement,
    Insets as WindUiInsets, Role as WindUiRole, Sender as WindUiSender, Signal as WindUiSignal,
    Truncate as WindUiTruncate,
};

use super::design::{SPACE_MD, SPACE_SM, SPACE_XL, TEXT_2XL, TEXT_SM, TEXT_XS, WEIGHT_SEMIBOLD};
use super::fcitx_config::{
    engine_get, engine_set, parse_addons, parse_config, parse_input_methods, AddonEntry,
    ConfigOption, InputMethodEntry, OptionKind, Tree, ADDONS_URI, INPUT_METHODS_URI,
};
use super::settings_locale::LocaleCatalog;

/// Quiet time after the last edit before it is saved, so typing in a text
/// field or clicking a stepper several times sends one save.
const SAVE_DELAY: Duration = Duration::from_millis(450);
/// Saves the page can undo.
const UNDO_DEPTH: usize = 30;
/// Options owned by Settings' candidate count.
const PAGE_SIZE_OPTIONS: [&str; 2] = ["PageSize", "DefaultPageSize"];
/// Stepper bounds for an Integer option without IntMin or IntMax.
const INTEGER_RANGE: (f64, f64) = (-1_000_000.0, 1_000_000.0);

/// The editable control of one option, bound to a signal the page reads back.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Binding {
    Switch(WindUiSignal<bool>),
    Number(WindUiSignal<f64>),
    Choice(WindUiSignal<usize>),
    Text(WindUiSignal<String>),
    /// Keys separated by spaces.
    Keys(WindUiSignal<String>),
    /// One item per line.
    Lines(WindUiSignal<String>),
}

impl Binding {
    fn version(self) -> u64 {
        match self {
            Self::Switch(signal) => signal.version(),
            Self::Number(signal) => signal.version(),
            Self::Choice(signal) => signal.version(),
            Self::Text(signal) | Self::Keys(signal) | Self::Lines(signal) => signal.version(),
        }
    }

    fn dispose(self) {
        match self {
            Self::Switch(signal) => signal.dispose(),
            Self::Number(signal) => signal.dispose(),
            Self::Choice(signal) => signal.dispose(),
            Self::Text(signal) | Self::Keys(signal) | Self::Lines(signal) => signal.dispose(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum RowKind {
    /// A nested configuration's title; its options follow as rows.
    Heading,
    Edit {
        binding: Binding,
        /// Enum values in order, for `Binding::Choice`.
        values: Vec<String>,
        labels: Vec<String>,
        range: (f64, f64),
    },
    SubConfig(String),
    Unsupported,
}

#[derive(Clone, Debug, PartialEq)]
struct Row {
    /// Unique within one load: the load number and the option path.
    key: String,
    path: Vec<String>,
    depth: usize,
    label: String,
    tooltip: String,
    kind: RowKind,
    default: Tree,
    /// The control's version when the page loaded: a row the user has not
    /// touched is never written back, even where its control cannot show
    /// the stored value exactly.
    loaded_version: u64,
}

fn option_label(option: &ConfigOption) -> String {
    if option.description.is_empty() {
        option.name.clone()
    } else {
        option.description.clone()
    }
}

fn joined_items(tree: Option<&Tree>, separator: &str) -> String {
    tree.map(Tree::list_items)
        .unwrap_or_default()
        .join(separator)
}

fn edit_kind(kind: &OptionKind, value: Option<&Tree>) -> Option<RowKind> {
    let text = value.map_or("", |tree| tree.value.as_str());
    let none = (Vec::new(), Vec::new(), (0.0, 0.0));
    let (binding, (values, labels, range)) = match kind {
        OptionKind::Boolean => (Binding::Switch(windui_signal(text == "True")), none),
        OptionKind::Integer { min, max } => {
            let range = (
                min.map_or(INTEGER_RANGE.0, |min| min as f64),
                max.map_or(INTEGER_RANGE.1, |max| max as f64),
            );
            let number = text
                .parse::<f64>()
                .unwrap_or(range.0)
                .clamp(range.0, range.1);
            (
                Binding::Number(windui_signal(number)),
                (Vec::new(), Vec::new(), range),
            )
        }
        OptionKind::Enum { values, labels } => {
            let index = values.iter().position(|value| value == text).unwrap_or(0);
            (
                Binding::Choice(windui_signal(index)),
                (values.clone(), labels.clone(), (0.0, 0.0)),
            )
        }
        OptionKind::String | OptionKind::Key => {
            (Binding::Text(windui_signal(text.to_owned())), none)
        }
        OptionKind::KeyList => (Binding::Keys(windui_signal(joined_items(value, " "))), none),
        OptionKind::StringList => (
            Binding::Lines(windui_signal(joined_items(value, "\n"))),
            none,
        ),
        OptionKind::Group | OptionKind::External { .. } | OptionKind::Unsupported(_) => {
            return None
        }
    };
    Some(RowKind::Edit {
        binding,
        values,
        labels,
        range,
    })
}

fn flatten(
    options: &[ConfigOption],
    value: &Tree,
    load: u64,
    parent: &[String],
    depth: usize,
    rows: &mut Vec<Row>,
) {
    for option in options {
        // The candidate count on the Appearance page sets these for every
        // input method, and the engine applies it over them.
        if PAGE_SIZE_OPTIONS.contains(&option.name.as_str()) {
            continue;
        }
        let mut path = parent.to_vec();
        path.push(option.name.clone());
        let current = value.get_path(&path);
        let kind = match &option.kind {
            OptionKind::Group => RowKind::Heading,
            OptionKind::External {
                uri,
                sub_config: true,
            } => RowKind::SubConfig(uri.clone()),
            // A native configuration tool Settings cannot open.
            OptionKind::External { .. } => continue,
            OptionKind::Unsupported(_) => RowKind::Unsupported,
            kind => edit_kind(kind, current).unwrap_or(RowKind::Unsupported),
        };
        let loaded_version = match &kind {
            RowKind::Edit { binding, .. } => binding.version(),
            _ => 0,
        };
        rows.push(Row {
            loaded_version,
            key: format!("{load}:{}", path.join("/")),
            path: path.clone(),
            depth,
            label: option_label(option),
            tooltip: option.tooltip.clone(),
            kind: kind.clone(),
            default: option.default.clone(),
        });
        if kind == RowKind::Heading {
            flatten(&option.children, value, load, &path, depth + 1, rows);
        }
    }
}

/// The value tree a row's control holds now.
fn row_value(row: &Row) -> Option<Tree> {
    let RowKind::Edit {
        binding, values, ..
    } = &row.kind
    else {
        return None;
    };
    let mut tree = Tree::default();
    match *binding {
        Binding::Switch(signal) => {
            tree.value = if signal.get() { "True" } else { "False" }.to_owned()
        }
        Binding::Number(signal) => tree.value = (signal.get().round() as i64).to_string(),
        Binding::Choice(signal) => tree.value = values.get(signal.get())?.clone(),
        Binding::Text(signal) => tree.value = signal.get(),
        Binding::Keys(signal) => tree.set_list_items(
            &signal
                .get()
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        ),
        Binding::Lines(signal) => tree.set_list_items(
            &signal
                .get()
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        ),
    }
    Some(tree)
}

/// Puts `value` into a row's control.
fn show_value(row: &Row, value: &Tree) {
    let RowKind::Edit {
        binding,
        values,
        range,
        ..
    } = &row.kind
    else {
        return;
    };
    match *binding {
        Binding::Switch(signal) => signal.set(value.value == "True"),
        Binding::Number(signal) => signal.set(
            value
                .value
                .parse::<f64>()
                .unwrap_or(range.0)
                .clamp(range.0, range.1),
        ),
        Binding::Choice(signal) => signal.set(
            values
                .iter()
                .position(|item| *item == value.value)
                .unwrap_or(0),
        ),
        Binding::Text(signal) => signal.set(value.value.clone()),
        Binding::Keys(signal) => signal.set(value.list_items().join(" ")),
        Binding::Lines(signal) => signal.set(value.list_items().join("\n")),
    }
}

/// What the page asked the engine for, sent back to the UI thread.
pub(crate) enum FcitxConfigMessage {
    Loaded {
        load: u64,
        result: Result<String, String>,
    },
    Saved {
        load: u64,
        result: Result<(), String>,
    },
    Addons(Result<String, String>),
    InputMethods(Result<String, String>),
}

/// Engine work, run in order on one thread so saves reach the engine in the
/// order they were made.
type EngineJob = Box<dyn FnOnce(&Path) -> FcitxConfigMessage + Send>;

/// One open configuration: what Settings shows on the configuration page.
#[derive(Clone, Debug, PartialEq)]
struct Location {
    uri: String,
    title: String,
}

/// The configuration page and the engine's addon and input method lists.
#[derive(Clone, Copy)]
pub(crate) struct FcitxConfigPages {
    install_root: WindUiSignal<PathBuf>,
    jobs: WindUiSignal<Option<mpsc::Sender<EngineJob>>>,
    location: WindUiSignal<Option<Location>>,
    /// Pages opened before the current one, for Back.
    back: WindUiSignal<Vec<Location>>,
    rows: WindUiSignal<Vec<Row>>,
    /// Rows of earlier loads, disposed on the next load once nothing draws them.
    retired: WindUiSignal<Vec<Row>>,
    /// The values the engine holds, as far as this page knows.
    saved: WindUiSignal<Tree>,
    undo: WindUiSignal<Vec<Tree>>,
    /// The next save comes from Undo and is not itself undoable.
    undoing: WindUiSignal<bool>,
    load: WindUiSignal<u64>,
    loading: WindUiSignal<bool>,
    status: WindUiSignal<String>,
    pub(crate) addons: WindUiSignal<Vec<AddonEntry>>,
    pub(crate) input_methods: WindUiSignal<Vec<InputMethodEntry>>,
    lists_error: WindUiSignal<String>,
}

impl FcitxConfigPages {
    pub(crate) fn new(app: &mut WindUiApp, install_root: PathBuf, locale: LocaleCatalog) -> Self {
        let pages = Self {
            install_root: windui_signal(install_root),
            jobs: windui_signal(None),
            location: windui_signal(None),
            back: windui_signal(Vec::new()),
            rows: windui_signal(Vec::new()),
            retired: windui_signal(Vec::new()),
            saved: windui_signal(Tree::default()),
            undo: windui_signal(Vec::new()),
            undoing: windui_signal(false),
            load: windui_signal(0),
            loading: windui_signal(false),
            status: windui_signal(String::new()),
            addons: windui_signal(Vec::new()),
            input_methods: windui_signal(Vec::new()),
            lists_error: windui_signal(String::new()),
        };
        let sender: WindUiSender<FcitxConfigMessage> =
            app.channel(move |ctx, message| pages.receive(ctx, message, &locale));
        let (jobs, queue) = mpsc::channel::<EngineJob>();
        let root = pages.install_root.get();
        thread::spawn(move || {
            for job in queue {
                if sender.send(job(&root)).is_err() {
                    break;
                }
            }
        });
        pages.jobs.set(Some(jobs));
        super::save_on_change(
            move || pages.rows.with(|rows| rows_version(rows)),
            SAVE_DELAY,
            move |_| pages.flush(),
        );
        pages
    }

    fn spawn(self, job: impl FnOnce(&Path) -> FcitxConfigMessage + Send + 'static) {
        self.jobs.with(|jobs| {
            if let Some(jobs) = jobs {
                let _ = jobs.send(Box::new(job));
            }
        });
    }

    /// Reads the engine's addon and input method lists again.
    pub(crate) fn refresh_lists(self) {
        self.spawn(|root| FcitxConfigMessage::Addons(engine_get(root, ADDONS_URI)));
        self.spawn(|root| FcitxConfigMessage::InputMethods(engine_get(root, INPUT_METHODS_URI)));
    }

    /// Why the addon list is empty: the last error, or `empty`.
    pub(crate) fn lists_error_or_empty(self, empty: String) -> WindUiSignal<String> {
        self.lists_error.map(move |error| {
            if error.is_empty() {
                empty.clone()
            } else {
                error.clone()
            }
        })
    }

    /// Whether the engine runs addon `id` and it has options.
    pub(crate) fn addon_configurable(self, id: &str) -> bool {
        self.addons.with(|addons| {
            addons
                .iter()
                .any(|addon| addon.id == id && addon.configurable && addon.loaded)
        })
    }

    /// Whether the engine has options for input method `id`.
    pub(crate) fn input_method_configurable(self, id: &str) -> bool {
        self.input_methods.with(|entries| {
            entries
                .iter()
                .any(|entry| entry.id == id && entry.configurable)
        })
    }

    pub(crate) fn is_open(self) -> bool {
        self.location.with(Option::is_some)
    }

    /// Opens `uri` as the configuration page, from the list pages.
    pub(crate) fn open(self, uri: String, title: String) {
        self.back.set(Vec::new());
        self.show(Location { uri, title });
    }

    pub(crate) fn close(self) {
        self.flush();
        self.location.set(None);
        self.back.set(Vec::new());
        self.retire_rows();
    }

    fn open_sub(self, uri: String, title: String) {
        self.flush();
        if let Some(current) = self.location.get() {
            self.back.update(|back| back.push(current));
        }
        self.show(Location { uri, title });
    }

    fn go_back(self) {
        self.flush();
        let mut previous = None;
        self.back.update(|back| previous = back.pop());
        match previous {
            Some(location) => self.show(location),
            None => self.close(),
        }
    }

    fn retire_rows(self) {
        let rows = self.rows.get();
        self.rows.set(Vec::new());
        self.retired.update(|retired| retired.extend(rows));
    }

    fn show(self, location: Location) {
        self.retire_rows();
        self.saved.set(Tree::default());
        self.undo.set(Vec::new());
        self.loading.set(true);
        self.status.set(String::new());
        let load = self.load.get() + 1;
        self.load.set(load);
        let uri = location.uri.clone();
        self.location.set(Some(location));
        self.spawn(move |root| FcitxConfigMessage::Loaded {
            load,
            result: engine_get(root, &uri),
        });
    }

    fn reload(self) {
        if let Some(location) = self.location.get() {
            self.show(location);
        }
    }

    fn receive(self, ctx: &mut EventCtx, message: FcitxConfigMessage, locale: &LocaleCatalog) {
        match message {
            FcitxConfigMessage::Loaded { load, result } => {
                if load != self.load.get() {
                    return;
                }
                self.loading.set(false);
                // The rows drawn before this load are gone from the tree by now.
                for row in self.retired.get() {
                    if let RowKind::Edit { binding, .. } = row.kind {
                        binding.dispose();
                    }
                }
                self.retired.set(Vec::new());
                match result.and_then(|json| parse_config(&json)) {
                    Ok(config) => {
                        let mut rows = Vec::new();
                        flatten(&config.options, &config.value, load, &[], 0, &mut rows);
                        self.rows.set(rows);
                        self.saved.set(config.value);
                    }
                    Err(error) => self.status.set(locale.format(
                        "settings.fcitx.load_failed",
                        &[("error", &locale.text(&error))],
                    )),
                }
            }
            FcitxConfigMessage::Saved { load, result } => {
                let current = load == self.load.get();
                match result {
                    Ok(()) if current => self.status.set(locale.text("settings.fcitx.saved")),
                    Ok(()) => {}
                    Err(error) => {
                        // Also for a page already left: its last edit was lost.
                        let message = locale.format(
                            "settings.fcitx.save_failed",
                            &[("error", &locale.text(&error))],
                        );
                        ctx.toast_err(message.clone());
                        if current {
                            // Show what the engine really holds.
                            self.reload();
                            self.status.set(message);
                        }
                    }
                }
            }
            FcitxConfigMessage::Addons(result) => {
                match result.and_then(|json| parse_addons(&json)) {
                    Ok(addons) => {
                        self.addons.set(addons);
                        self.lists_error.set(String::new());
                    }
                    Err(error) => self.lists_error.set(locale.text(&error)),
                }
            }
            FcitxConfigMessage::InputMethods(result) => {
                if let Ok(entries) = result.and_then(|json| parse_input_methods(&json)) {
                    self.input_methods.set(entries);
                }
            }
        }
    }

    /// Sends what changed since the last save, now.
    fn flush(self) {
        let Some(location) = self.location.get() else {
            return;
        };
        let saved = self.saved.get();
        let mut next = saved.clone();
        let mut changes = Tree::default();
        self.rows.with(|rows| {
            for row in rows {
                let touched = match &row.kind {
                    RowKind::Edit { binding, .. } => binding.version() != row.loaded_version,
                    _ => false,
                };
                let Some(value) = row_value(row).filter(|_| touched) else {
                    continue;
                };
                if saved.get_path(&row.path) != Some(&value) {
                    *changes.entry_path(&row.path) = value.clone();
                    *next.entry_path(&row.path) = value;
                }
            }
        });
        if changes.children.is_empty() {
            self.undoing.set(false);
            return;
        }
        if self.undoing.get() {
            self.undoing.set(false);
        } else {
            self.undo.update(|undo| {
                undo.push(saved);
                if undo.len() > UNDO_DEPTH {
                    undo.remove(0);
                }
            });
        }
        self.saved.set(next);
        let load = self.load.get();
        let uri = location.uri;
        self.spawn(move |root| FcitxConfigMessage::Saved {
            load,
            result: engine_set(root, &uri, &changes),
        });
    }

    fn undo(self) {
        let mut previous = None;
        self.undo.update(|undo| previous = undo.pop());
        let Some(previous) = previous else {
            return;
        };
        self.undoing.set(true);
        self.rows.with(|rows| {
            for row in rows {
                if let Some(value) = previous.get_path(&row.path) {
                    show_value(row, value);
                }
            }
        });
    }

    fn reset_all(self) {
        self.rows.with(|rows| {
            for row in rows {
                show_value(row, &row.default);
            }
        });
    }

    /// The configuration page: a title bar with Back, Undo and Reset, then
    /// one row per option.
    pub(crate) fn page(self, locale: &LocaleCatalog) -> WindUiElement {
        let title = self.location.map(|location| {
            location
                .as_ref()
                .map_or_else(String::new, |location| location.title.clone())
        });
        let header = WindUiElement::row()
            .width_match()
            .cross(WindUiAlign::Center)
            .spacing(SPACE_SM)
            .child(
                WindUiElement::button(locale.text("settings.fcitx.back"))
                    .small()
                    .outline()
                    .neutral()
                    .on_click(move |_| self.go_back()),
            )
            .child(
                WindUiElement::label_signal(title)
                    .font_size(TEXT_2XL)
                    .font_weight(WEIGHT_SEMIBOLD)
                    .fg_role(WindUiRole::Text)
                    .weight(1.0)
                    .max_lines(1)
                    .truncate(WindUiTruncate::End),
            )
            .child(
                WindUiElement::button(locale.text("settings.fcitx.undo"))
                    .small()
                    .outline()
                    .neutral()
                    .enabled_when(move || self.undo.with(|undo| !undo.is_empty()))
                    .on_click(move |_| self.undo()),
            )
            .child(
                WindUiElement::button(locale.text("settings.fcitx.reset_all"))
                    .small()
                    .outline()
                    .neutral()
                    .enabled_when(move || self.rows.with(|rows| !rows.is_empty()))
                    .on_click(move |_| self.reset_all()),
            );
        let loading_text = locale.text("settings.fcitx.loading");
        let status = WindUiElement::row()
            .width_match()
            .cross(WindUiAlign::Center)
            .spacing(SPACE_SM)
            .child(
                WindUiElement::label(loading_text)
                    .font_size(TEXT_XS)
                    .fg_role(WindUiRole::TextMuted)
                    .visible_when(move || self.loading.get()),
            )
            .child(
                WindUiElement::label_signal(self.status)
                    .font_size(TEXT_XS)
                    .fg_role(WindUiRole::TextMuted)
                    .weight(1.0)
                    .visible_when(move || !self.loading.get()),
            )
            .child(
                WindUiElement::button(locale.text("settings.fcitx.retry"))
                    .small()
                    .outline()
                    .neutral()
                    .visible_when(move || !self.loading.get() && self.rows.with(Vec::is_empty))
                    .on_click(move |_| self.reload()),
            );
        let row_locale = locale.clone();
        let list = WindUiElement::list_signal(
            self.rows,
            |row| row.key.clone(),
            move |row| self.row(row, &row_locale),
        )
        .width_match()
        .weight(1.0);
        WindUiElement::col()
            .fill()
            .padding(SPACE_XL)
            .spacing(SPACE_MD)
            .child(header)
            .child(status)
            .child(list)
    }

    fn row(self, row: Row, locale: &LocaleCatalog) -> WindUiElement {
        let indent = (row.depth * 16) as i32;
        let control = match &row.kind {
            RowKind::Heading => {
                return WindUiElement::row()
                    .width_match()
                    .padding_edges(WindUiInsets::new(12 + indent / 2, 18, 12, 6))
                    .child(
                        WindUiElement::label(row.label.clone())
                            .font_size(TEXT_SM)
                            .font_weight(WEIGHT_SEMIBOLD)
                            .fg_role(WindUiRole::Text),
                    );
            }
            RowKind::SubConfig(uri) => {
                let (uri, title) = (uri.clone(), row.label.clone());
                WindUiElement::button(locale.text("settings.fcitx.open"))
                    .small()
                    .outline()
                    .on_click(move |_| self.open_sub(uri.clone(), title.clone()))
            }
            RowKind::Unsupported => WindUiElement::label(locale.text("settings.fcitx.unsupported"))
                .font_size(TEXT_XS)
                .fg_role(WindUiRole::TextMuted),
            RowKind::Edit {
                binding,
                labels,
                range,
                ..
            } => {
                let editor = match *binding {
                    Binding::Switch(signal) => WindUiElement::switch(signal),
                    Binding::Number(signal) => {
                        WindUiElement::stepper(signal, range.0, range.1, 1.0)
                    }
                    Binding::Choice(signal) => {
                        WindUiElement::dropdown(labels.clone(), signal).width(220)
                    }
                    Binding::Text(signal) => WindUiElement::text_input(signal, "").width(220),
                    Binding::Keys(signal) => {
                        WindUiElement::text_input(signal, locale.text("settings.fcitx.keys_hint"))
                            .width(260)
                    }
                    Binding::Lines(signal) => {
                        WindUiElement::text_input(signal, locale.text("settings.fcitx.lines_hint"))
                            .multiline()
                            .width(260)
                            .height(96)
                    }
                };
                let reset_row = row.clone();
                WindUiElement::row()
                    .cross(WindUiAlign::Center)
                    .spacing(SPACE_SM)
                    .child(editor)
                    .child(
                        WindUiElement::button("\u{21BA}")
                            .small()
                            .outline_soft()
                            .neutral()
                            .tooltip(locale.text("settings.fcitx.reset"))
                            .on_click(move |_| show_value(&reset_row, &reset_row.default)),
                    )
            }
        };
        let line = if row.tooltip.is_empty() {
            WindUiElement::setting_row(row.label.clone(), control)
        } else {
            WindUiElement::setting_row_desc(row.label.clone(), row.tooltip.clone(), control)
        };
        WindUiElement::row()
            .width_match()
            .padding_edges(WindUiInsets::new(12 + indent, 4, 12, 4))
            .child(line.weight(1.0))
    }
}

fn rows_version(rows: &[Row]) -> u64 {
    rows.iter()
        .filter_map(|row| match row.kind {
            RowKind::Edit { binding, .. } => Some(binding.version()),
            _ => None,
        })
        .fold(0u64, u64::wrapping_add)
}
