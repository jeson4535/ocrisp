use crate::abstractions::*;
use crate::db::DBTrait;
use crate::db::data::Data;
use qdrant_client::Qdrant;
use std::time::Instant;

pub fn run_gui() {
    let _handle = Qdrant::run_db(false).unwrap();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 220.0])
            .with_max_inner_size([301.0, 221.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-350.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "ocrisp",
        native_options,
        Box::new(|cc| Ok(Box::new(Gui::new(cc)))),
    );
}

pub struct Gui {
    dbs: Vec<String>,
    db_selected: usize,
    ais: Vec<AI>,
    ai_selected: usize,
    db: Option<Qdrant>,
    pdf_n: u64,
    timer: Option<Instant>,
    message: String,
    proc_state: ProcessingState,
}

struct ProcessingState {
    pdf_name: String,
    progress_bar: f32,
    is: bool,
    receiver: Option<tokio::sync::mpsc::UnboundedReceiver<(String, f32, bool)>>,
    // name_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
}

impl ProcessingState {
    fn new() -> Self {
        Self {
            pdf_name: "".to_owned(),
            progress_bar: 0.0,
            is: false,
            receiver: None,
            // name_receiver: None,
        }
    }
}

impl Default for Gui {
    fn default() -> Self {
        let dbs = vec!["Qdrant".to_owned()];
        let db_selected = 0;
        let gemmaendpoint = AI::new("http://localhost:11434/api/embed", "embeddinggemma", 768);
        let ais = vec![gemmaendpoint];
        let pdf_n = Data::count_pdfs();

        let ai_selected = 0;
        Self {
            dbs,
            db_selected,
            ais,
            ai_selected,
            db: None,
            pdf_n,
            timer: None,
            message: "".to_owned(),
            proc_state: ProcessingState::new(),
        }
    }
}

impl Gui {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        // }
        Default::default()
    }

    pub fn mcp_install_widget(&mut self, ui: &mut egui::Ui) {
        // This two values have to be decided at app startup
        let claude_desktop_found: bool = false;
        let claude_mcp_not_installed: bool = false;

        if claude_desktop_found && claude_mcp_not_installed {
            ui.vertical_centered(|ui| {
                ui.separator();
                ui.label("MCP Hosts");
                if ui.button("Install to Claude Desktop").clicked() {}
            });
        }
    }

    fn handle_receiver(&mut self) {
        if let Some(rx) = &mut self.proc_state.receiver {
            let mut updates = Vec::new();
            while let Ok((name, progress, is_complete)) = rx.try_recv() {
                updates.push((name, progress, is_complete));
            }

            for (name, progress, is_complete) in updates {
                if !is_complete {
                    self.proc_state.progress_bar = progress;
                    self.proc_state.pdf_name = name;
                } else {
                    self.proc_state.is = false;
                    self.message = "Embed is completed".to_owned();
                    self.timer = Some(Instant::now());
                }
            }
        }
    }

    fn show_timed_message(
        time: &mut Option<std::time::Instant>,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        message: &str,
    ) {
        if let Some(start_time) = *time {
            if start_time.elapsed().as_secs_f32() < 3.0 {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(message);
                });
                ctx.request_repaint();
            } else {
                *time = None;
            }
        }
    }

    fn show_message(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let message = &self.message;
        let time = &mut self.timer;
        Gui::show_timed_message(time, ui, ctx, message);
    }

    pub fn embed_widget(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("Process");
        ui.horizontal(|ui| {
            if ui.button("Embed everything").clicked() {
                self.db = Some(Qdrant::init(None).unwrap());
                self.proc_state.is = true;
                if let Some(db) = self.db.clone() {
                    let endpoint = self.ais[0].clone();
                    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                    self.proc_state.receiver = Some(rx);
                    tokio::spawn(async move {
                        // If we don't have a table for this AI model, we'll create it
                        let _ = db
                            .create_table(&endpoint.model, endpoint.dims as u64)
                            .await
                            .ok();

                        let pdfs = Data::list_pdfs();
                        for pdf in pdfs {
                            let has_it = db
                                .has_pdf(&endpoint.model, &pdf, endpoint.dims)
                                .await
                                .unwrap();
                            // if we have it already in the database, we won't embed again, this is important
                            if !has_it {
                                let chunks = Chunk::from_pdf(&pdf).unwrap();
                                let n = chunks.len();
                                for (i, chunk) in chunks.iter().enumerate() {
                                    let embed = chunk.embed(&endpoint).await.unwrap();
                                    // println!("{:?}",embed);
                                    let d = db.post(&endpoint.model, embed).await.unwrap();
                                    println!("{:?}",d);
                                    let progress: f32 = (i as f32 + 1.0) / n as f32;
                                    if tx
                                        .send((pdf.to_str().unwrap().to_owned(), progress, false))
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                            }
                        }

                        tx.send(("".to_owned(), 0.0f32, true)).ok();
                    });
                }
            }
        });
    }

    pub fn select_db_widget(&mut self, ui: &mut egui::Ui) {
        ui.label("Select Database");
        egui::ComboBox::from_id_salt("Database")
            .selected_text(self.dbs[self.db_selected].clone())
            .show_ui(ui, |ui| {
                for (i, db) in self.dbs.iter().enumerate() {
                    ui.selectable_value(&mut self.db_selected, i, db);
                }
            });
    }

    pub fn select_ai_widget(&mut self, ui: &mut egui::Ui) {
        ui.label("Select AI");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("AI")
                .selected_text(&self.ais[self.ai_selected].model.clone())
                .show_ui(ui, |ui| {
                    for (i, db) in self.ais.iter().enumerate() {
                        ui.add_enabled_ui(true, |ui| {
                            ui.selectable_value(&mut self.ai_selected, i, &db.model);
                        });
                    }
                });

            if ui.button("+").clicked() {}
        });
    }

    pub fn pdf_founds(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.vertical_centered(|ui| {
            let text = format!("PDFs found: {}", self.pdf_n);
            ui.label(text);
        });
    }

    pub fn proc_widget(&mut self, ui: &mut egui::Ui) {
        if self.proc_state.is {
            let text = format!("Processing: {}", self.proc_state.pdf_name);
            ui.label(text);
            ui.add(
                egui::ProgressBar::new(self.proc_state.progress_bar)
                    .show_percentage()
                    .animate(true),
            );
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.select_db_widget(ui);

            ui.separator();

            self.select_ai_widget(ui);

            self.embed_widget(ui);

            self.mcp_install_widget(ui);

            self.pdf_founds(ui);

            self.proc_widget(ui);

            self.show_message(ui, ctx);

            self.handle_receiver();
        });
    }
}
