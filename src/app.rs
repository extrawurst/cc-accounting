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
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    show_hidden: bool,
    row_meta_data: Vec<RowMetaData>,

    #[serde(skip)]
    rows: Vec<CsvRow>,
    #[serde(skip)]
    visible_rows: usize,
    #[serde(skip)]
    max_cells: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            show_hidden: false,
            visible_rows: 0,
            max_cells: 0,
            rows: Vec::new(),
            row_meta_data: Vec::new(),
        }
    }
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
            ..base
        };

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
                    if ui.checkbox(&mut self.show_hidden, "Show Hidden").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            use egui_extras::{Size, TableBuilder};

            TableBuilder::new(ui)
                .striped(true)
                .columns(Size::initial(40.0).at_least(40.0), self.max_cells + 1)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .resizable(true)
                .body(|body| {
                    let rows = if self.show_hidden {
                        self.rows.len()
                    } else {
                        self.visible_rows
                    };

                    let mut rows_skipped = 0;
                    body.rows(16.0, rows, |row_index, mut row| {
                        if !self.show_hidden {
                            while self.row_meta_data[row_index + rows_skipped].hidden {
                                rows_skipped = rows_skipped + 1;
                            }
                        }
                        let meta = &mut self.row_meta_data[row_index + rows_skipped];

                        let mut update_hidden = false;
                        row.col(|ui| {
                            if self.show_hidden {
                                update_hidden = ui.checkbox(&mut meta.hidden, "hide").changed();
                            } else {
                                if ui.small_button("hide").clicked() {
                                    meta.hidden = true;
                                    update_hidden = true;
                                }
                            }
                        });
                        if update_hidden {
                            self.update_hidden();
                        }

                        for cell in &self.rows[row_index + rows_skipped].cells {
                            row.col(|ui| {
                                ui.label(cell);
                            });
                        }
                    });
                });
        });
    }
}
