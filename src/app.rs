use std::path::{Path, PathBuf};

use eframe::epaint::{self};
use egui::{
    color, plot::Text, Color32, CursorIcon, Id, InnerResponse, Label, LayerId, Order, Rect, Sense,
    Shape, Ui, Vec2, WidgetText,
};

const APP_KEY: &str = "CC";

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct CsvRow {
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct RowMetaData {
    pub hidden: bool,
    pub receipt: Option<String>,
}

impl RowMetaData {
    pub fn rename_pdf(&mut self, row: &CsvRow) {
        let target_name = self.target_file_name(row);
        if let Some(receipt) = self.receipt.as_mut() {
            let target_name = target_name.expect("cannot happen since receipt is not none");

            tracing::debug!("rename pdf: '{}' -> '{}'", receipt, target_name);

            std::fs::rename(receipt.clone(), target_name.clone()).expect("TODO");
            *receipt = target_name;
        }
    }

    fn is_name_correct(&self, row: &CsvRow) -> bool {
        let target_name = self.target_file_name(row);
        if let Some(receipt) = self.receipt.as_ref() {
            target_name.map(|f| f == *receipt).unwrap_or(false)
        } else {
            // no receipt means the name is correct
            true
        }
    }

    fn target_file_name(&self, row: &CsvRow) -> Option<String> {
        if let Some(receipt) = self.receipt.as_ref() {
            let receipt_path = Path::new(receipt);
            let target_name = format!(
                "{}/{}{}EUR-{}.pdf",
                receipt_path.parent().unwrap().to_str().unwrap(),
                row.cells[0],
                row.cells[3],
                row.cells[2],
            );

            Some(target_name)
        } else {
            None
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    show_hidden: bool,
    row_meta_data: Vec<RowMetaData>,

    #[serde(skip)]
    rows: Vec<CsvRow>,
    #[serde(skip)]
    pdfs: Vec<PathBuf>,
    #[serde(skip)]
    visible_rows: usize,
    #[serde(skip)]
    max_cells: usize,

    #[serde(skip)]
    drop_row: Option<usize>,
    #[serde(skip)]
    drag_row: Option<usize>,
}

fn find_pdfs(path: &str) -> Vec<PathBuf> {
    let paths = std::fs::read_dir(path).unwrap();

    let mut res = Vec::new();
    for path in paths {
        let path = path.unwrap().path();
        if path.extension().map(|ext| ext == "pdf").unwrap_or_default() {
            res.push(path.to_path_buf());
        }
    }

    res
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let base: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let file = std::fs::File::open("./cc-2022-06/table.csv").unwrap();
        let mut rdr = csv::ReaderBuilder::new()
            .flexible(true)
            .delimiter(b';')
            .from_reader(file);
        let mut rows = Vec::new();
        let mut max_cells = 0;
        for result in rdr.byte_records() {
            let result = result.unwrap();
            let mut row = Vec::new();
            for result in result.iter() {
                row.push(String::from_utf8_lossy(result).to_string());
            }

            max_cells = std::cmp::max(max_cells, row.len());

            rows.push(CsvRow { cells: row });
        }

        let row_count = rows.len();

        let mut app = Self {
            rows,
            max_cells,
            pdfs: Vec::new(),
            ..base
        };

        app.reread_pdfs();

        //if mismatch in length we regenerate meta data
        if app.row_meta_data.len() < app.rows.len() {
            app.row_meta_data = vec![RowMetaData::default(); row_count];
        }

        app.update_hidden();

        app
    }

    fn update_hidden(&mut self) {
        self.visible_rows = self.row_meta_data.iter().filter(|r| !r.hidden).count();
        // info!("update hidden: {}", self.visible_rows);
    }

    fn check_drop(&mut self) {
        if let Some(source_row) = self.drag_row {
            if let Some(drop_row) = self.drop_row {
                if let Some(meta) = self.row_meta_data.get_mut(drop_row) {
                    meta.receipt = Some(self.pdfs[source_row].to_string_lossy().to_string());
                    self.drag_row = None;
                    self.drop_row = None;
                    self.reread_pdfs();
                }
            }
        }
    }

    fn reread_pdfs(&mut self) {
        self.pdfs = find_pdfs("./cc-2022-06");

        // info!("found pdfs: {}", self.pdfs.len());

        self.pdfs = self
            .pdfs
            .iter()
            .filter(|p| {
                !self.row_meta_data.iter().any(|e| match &e.receipt {
                    None => false,
                    Some(e) => {
                        let match_found = p.to_str().map_or(false, |p| p == e);
                        // info!("found match: {:?} / '{}'", p, e);
                        match_found
                    }
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        // info!("pdfs after filter: {}", self.pdfs.len());
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Clear All").clicked() {
                        self.row_meta_data.iter_mut().for_each(|e| e.receipt = None);
                        self.reread_pdfs();
                        ui.close_menu();
                    }
                    if ui.checkbox(&mut self.show_hidden, "Show Hidden").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::SidePanel::right("right_panel")
                .resizable(true)
                .default_width(150.0)
                .width_range(80.0..=200.0)
                .show_inside(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Files");
                    });
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let id_source = "my_drag_and_drop_demo";
                        for (idx, pdf) in self.pdfs.iter().enumerate() {
                            let item_id = Id::new(id_source).with(idx);
                            App::drag_source(ui, item_id, |ui| {
                                ui.label(pdf.to_str().unwrap());
                            });

                            if ui.memory().is_being_dragged(item_id) {
                                self.drag_row = Some(idx);
                            }
                        }
                    });
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                self.draw_table(ui, ctx);
            });
        });
    }
}

impl App {
    pub fn drop_target<R>(
        ui: &mut Ui,
        can_accept_what_is_being_dragged: bool,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let is_being_dragged = ui.memory().is_anything_being_dragged();

        let margin = Vec2::splat(4.0);

        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(margin);
        let where_to_put_background = ui.painter().add(Shape::Noop);
        let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
        let ret = body(&mut content_ui);
        let outer_rect =
            Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
        let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

        let style = if is_being_dragged && can_accept_what_is_being_dragged && response.hovered() {
            ui.visuals().widgets.active
        } else {
            ui.visuals().widgets.inactive
        };

        let mut fill = style.bg_fill;
        let mut stroke = style.bg_stroke;
        if is_being_dragged && !can_accept_what_is_being_dragged {
            // gray out:
            fill = color::tint_color_towards(fill, ui.visuals().window_fill());
            stroke.color = color::tint_color_towards(stroke.color, ui.visuals().window_fill());
        }

        ui.painter().set(
            where_to_put_background,
            epaint::RectShape {
                rounding: style.rounding,
                fill,
                stroke,
                rect,
            },
        );

        InnerResponse::new(ret, response)
    }

    pub fn drag_source(ui: &mut Ui, id: Id, body: impl FnOnce(&mut Ui)) {
        let is_being_dragged = ui.memory().is_being_dragged(id);

        if !is_being_dragged {
            let response = ui.scope(body).response;

            // Check for drags:
            let response = ui.interact(response.rect, id, Sense::drag());
            if response.hovered() {
                ui.output().cursor_icon = CursorIcon::Grab;
            }
        } else {
            ui.output().cursor_icon = CursorIcon::Grabbing;

            // Paint the body to a new layer:
            let layer_id = LayerId::new(Order::Tooltip, id);
            let response = ui.with_layer_id(layer_id, body).response;

            // Now we move the visuals of the body to where the mouse is.
            // Normally you need to decide a location for a widget first,
            // because otherwise that widget cannot interact with the mouse.
            // However, a dragged component cannot be interacted with anyway
            // (anything with `Order::Tooltip` always gets an empty [`Response`])
            // So this is fine!

            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos - response.rect.center();
                ui.ctx().translate_layer(layer_id, delta);
            }
        }
    }

    fn draw_table(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        use egui_extras::{Size, TableBuilder};

        let contains_pointer = ui.ui_contains_pointer();

        TableBuilder::new(ui)
            .striped(true)
            .columns(Size::initial(40.0).at_least(40.0), self.max_cells + 2)
            .cell_layout(
                egui::Layout::left_to_right(egui::Align::Center)
                    .with_cross_align(egui::Align::Center),
            )
            .resizable(true)
            .body(|body| {
                let rows = if self.show_hidden {
                    self.rows.len()
                } else {
                    self.visible_rows
                };

                let mut rows_skipped = 0;
                let row_height = 18.0;
                body.rows(row_height, rows, |row_index, mut row| {
                    let mut row_index = row_index + rows_skipped;
                    if !self.show_hidden {
                        while self.row_meta_data[row_index].hidden {
                            rows_skipped += 1;
                            row_index += 1;
                        }
                    }

                    let meta = &mut self.row_meta_data[row_index];

                    let mut update_hidden = false;
                    row.col(|ui| {
                        if self.show_hidden {
                            update_hidden = ui.checkbox(&mut meta.hidden, "hide").changed();
                        } else if ui.small_button("hide").clicked() {
                            meta.hidden = true;
                            update_hidden = true;
                        }
                    });

                    let is_hidden = meta.hidden;

                    if update_hidden {
                        self.update_hidden();
                    }

                    for cell in &self.rows[row_index].cells {
                        row.col(|ui| {
                            let row_hovered = contains_pointer
                                && ctx
                                    .pointer_hover_pos()
                                    .map(|pos| {
                                        let widget_pos = ui.next_widget_position().y;

                                        let cursor_height_div_2 = row_height / 2.0;
                                        pos.y > widget_pos - cursor_height_div_2
                                            && pos.y < widget_pos + cursor_height_div_2
                                    })
                                    .unwrap_or_default();

                            if is_hidden {
                                ui.style_mut().visuals.override_text_color = Some(Color32::GRAY);
                            }

                            let mut w = WidgetText::from(cell);
                            if row_hovered {
                                w = w.background_color(Color32::LIGHT_GREEN);
                            }
                            ui.label(w);
                            if is_hidden {
                                ui.reset_style();
                            }
                        });
                    }

                    let meta = &mut self.row_meta_data[row_index];
                    let csv_row = &self.rows[row_index];

                    let can_accept_what_is_being_dragged = meta.receipt.is_none();

                    let mut reread = false;
                    let is_receipt_name_correct = meta.is_name_correct(csv_row);

                    row.col(|ui| {
                        let response = match &meta.receipt {
                            Some(receipt) => {
                                let mut txt = WidgetText::from(receipt);
                                if !is_receipt_name_correct {
                                    txt = txt.color(Color32::RED);
                                }
                                ui.add(Label::new(txt).sense(Sense::click()))
                            }
                            None => {
                                Self::drop_target(ui, can_accept_what_is_being_dragged, |ui| {
                                    ui.label("-")
                                })
                                .response
                            }
                        };

                        let hovered_label = response.hovered();

                        response.context_menu(|ui| {
                            if ui.button("clear").clicked() {
                                meta.receipt = None;
                                reread = true;
                                ui.close_menu();
                            }
                            if ui
                                .add_enabled(meta.receipt.is_some(), egui::Button::new("name"))
                                .clicked()
                            {
                                meta.rename_pdf(&csv_row);
                                reread = true;
                                ui.close_menu();
                            }
                        });

                        let is_being_dragged = ui.memory().is_anything_being_dragged();
                        if is_being_dragged && can_accept_what_is_being_dragged && hovered_label {
                            self.drop_row = Some(row_index);
                        }
                    });

                    if reread {
                        self.reread_pdfs();
                    }
                });
            });

        if ui.input().pointer.any_released() {
            self.check_drop();
        }
    }
}
