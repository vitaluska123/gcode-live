use crate::app::state::AppState;
use crate::export::gcode as exporter;
use crate::MainWindow;
use slint::ComponentHandle;
use std::rc::Rc;
pub(crate) fn install_callbacks(main_window: &MainWindow, app_state: &AppState) {
    let board_bounds = app_state.preview_scene.board_bounds.clone();
    let frame_geometry = app_state.preview_scene.frame_geometry.clone();
    let source_home = app_state.source_home.clone();
    let source_file_stem = app_state.source_file_stem.clone();
    let current_settings = app_state.settings.clone();
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let weak_home = Rc::downgrade(&source_home);
    let weak_source_file_stem = Rc::downgrade(&source_file_stem);
    let window_weak = main_window.as_weak();

    main_window.on_export_tap(move || {
        let Some(window) = window_weak.upgrade() else { return; };
        let Some(board_rc) = weak_board.upgrade() else { return; };
        let Some(frame_rc) = weak_frame.upgrade() else { return; };
        let Some(settings_rc) = weak_settings.upgrade() else { return; };
        let Some(home_rc) = weak_home.upgrade() else { return; };
        let Some(source_file_stem_rc) = weak_source_file_stem.upgrade() else { return; };

        let board_borrow = board_rc.borrow();
        let Some(bounds) = board_borrow.as_ref() else {
            window.invoke_show_error("No board data loaded.".into());
            return;
        };

        let frame_borrow = frame_rc.borrow();
        let Some(frame) = frame_borrow.as_ref() else {
            window.invoke_show_error("No frame geometry calculated.".into());
            return;
        };

        let settings = settings_rc.borrow().clone();
        let (shift_x, shift_y) = settings.local_offset();
        let material_left = settings.material_offset_x - settings.material_width.max(0.0);
        let material_right = settings.material_offset_x;
        let material_bottom = settings.material_offset_y;
        let material_top = settings.material_offset_y + settings.material_height.max(0.0);
        let exceeds_material = frame.left + shift_x < material_left
            || frame.bottom + shift_y < material_bottom
            || frame.right + shift_x > material_right
            || frame.top + shift_y > material_top;
        if exceeds_material {
            let accepted = rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title("Ð Ð°Ð¼ÐºÐ° Ð²ÑÑÐ¾Ð´Ð¸Ñ Ð·Ð° ÑÐµÐºÑÑÐ¾Ð»Ð¸Ñ")
                .set_description(format!(
                    "Ð­ÐºÑÐ¿Ð¾ÑÑÐ¸ÑÑÐµÐ¼Ð°Ñ ÑÐ°Ð¼ÐºÐ° Ð²ÑÑÐ¾Ð´Ð¸Ñ Ð·Ð° Ð¿ÑÐµÐ´ÐµÐ»Ñ ÑÐµÐºÑÑÐ¾Ð»Ð¸ÑÐ° {:.1} Ã {:.1} Ð¼Ð¼. ÐÑÐ¾Ð´Ð¾Ð»Ð¶Ð¸ÑÑ ÑÐºÑÐ¿Ð¾ÑÑ?",
                    settings.material_width, settings.material_height
                ))
                .set_buttons(rfd::MessageButtons::OkCancel)
                .show();
            if !matches!(accepted, rfd::MessageDialogResult::Ok) { return; }
        } else {
            let edge_margin_x = settings.material_edge_margin_x.max(0.0);
            let edge_margin_y = settings.material_edge_margin_y.max(0.0);
            let too_close_to_edge = frame.left + shift_x < material_left + edge_margin_x
                || frame.right + shift_x > material_right - edge_margin_x
                || frame.bottom + shift_y < material_bottom + edge_margin_y
                || frame.top + shift_y > material_top - edge_margin_y;
            if too_close_to_edge {
                let accepted = rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Warning)
                    .set_title("ÐÐ»Ð°ÑÐ° ÑÐ»Ð¸ÑÐºÐ¾Ð¼ Ð±Ð»Ð¸Ð·ÐºÐ¾ Ðº ÐºÑÐ°Ñ")
                    .set_description(format!(
                        "Ð Ð°Ð¼ÐºÐ° Ð½Ð°ÑÐ¾Ð´Ð¸ÑÑÑ Ð±Ð»Ð¸Ð¶Ðµ Ð·Ð°Ð´Ð°Ð½Ð½Ð¾Ð³Ð¾ Ð¼Ð¸Ð½Ð¸Ð¼Ð°Ð»ÑÐ½Ð¾Ð³Ð¾ Ð¾ÑÑÑÑÐ¿Ð° {:.1} Ã {:.1} Ð¼Ð¼ Ð¾Ñ ÐºÑÐ°Ñ ÑÐµÐºÑÑÐ¾Ð»Ð¸ÑÐ°. ÐÑÐ¾Ð´Ð¾Ð»Ð¶Ð¸ÑÑ ÑÐºÑÐ¿Ð¾ÑÑ?",
                        edge_margin_x, edge_margin_y
                    ))
                    .set_buttons(rfd::MessageButtons::OkCancel)
                    .show();
                if !matches!(accepted, rfd::MessageDialogResult::Ok) { return; }
            }
        }

        let suggested_file_name = source_file_stem_rc
            .borrow()
            .as_deref()
            .map(|stem| format!("{stem}_frame.tap"))
            .unwrap_or_else(|| "frame.tap".to_owned());
        let Some(file_path) = rfd::FileDialog::new()
            .set_file_name(&suggested_file_name)
            .save_file()
        else {
            return;
        };

        let gcode = exporter::generate_frame_gcode(
            bounds,
            frame,
            &settings,
            *home_rc.borrow(),
        );

        if let Err(e) = exporter::save_gcode(&gcode, &file_path) {
            window.invoke_show_error(format!("Failed to export G-code: {}", e).into());
        }
    });
}
