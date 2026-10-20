//! A table the host page asks for (`deckcraft:insert-table`): the rows' texts go onto the current
//! slide as one `insert.table`, so one undo step removes it and the presentation counts as changed.

use crate::SlideApp;

/// Most rows, columns and characters per cell a request may carry.
pub const MAX_ROWS: usize = 50;
pub const MAX_COLS: usize = 20;
pub const MAX_CELL_CHARS: usize = 1000;

/// The usual row height (pt), and the share of the slide the table may cover.
const ROW_HEIGHT: f64 = 40.0;
const SLIDE_SHARE: f64 = 0.8;

/// `rows` as cell texts for `insert.table`, or why the request is refused. Line breaks and tabs
/// in a cell become spaces: a cell holds one line of text.
pub fn table_data(rows: &[Vec<String>]) -> Result<serde_json::Value, String> {
    if rows.is_empty() || rows.len() > MAX_ROWS {
        return Err(format!("a table needs 1 to {MAX_ROWS} rows"));
    }
    for row in rows {
        if row.is_empty() || row.len() > MAX_COLS {
            return Err(format!("a row needs 1 to {MAX_COLS} cells"));
        }
        if row.iter().any(|c| c.chars().count() > MAX_CELL_CHARS) {
            return Err(format!("a cell holds at most {MAX_CELL_CHARS} characters"));
        }
    }
    let clean = |c: &String| c.chars().map(|ch| if ch.is_control() { ' ' } else { ch }).collect::<String>();
    Ok(serde_json::json!(rows.iter().map(|r| r.iter().map(clean).collect::<Vec<_>>()).collect::<Vec<_>>()))
}

/// Insert the table in the middle of the current slide, squeezed to fit when there are many rows.
/// An `Err` leaves the presentation as it was.
pub fn insert_table(app: &mut SlideApp, rows: &[Vec<String>]) -> Result<(), String> {
    let data = table_data(rows)?;
    let cols = rows.iter().map(Vec::len).max().unwrap_or(1);
    let mut params = serde_json::json!({ "rows": rows.len(), "cols": cols, "data": data });
    let size = app.session.doc().map_err(|e| e.to_string())?.doc.slide_size;
    if size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0 {
        let (w, n) = (size.width * SLIDE_SHARE, rows.len() as f64);
        let h = n * ROW_HEIGHT.min(size.height * SLIDE_SHARE / n);
        params["rect"] = serde_json::json!([(size.width - w) / 2.0, (size.height - h) / 2.0, w, h]);
    }
    app.run("insert.table", params).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(r: &[&[&str]]) -> Vec<Vec<String>> {
        r.iter().map(|row| row.iter().map(|c| c.to_string()).collect()).collect()
    }

    fn app() -> SlideApp {
        SlideApp::new(deckcraft_engine::Session::with_new(), Default::default())
    }

    #[test]
    fn a_loop_table_goes_in_as_one_undoable_change() {
        let mut a = app();
        let table = |a: &SlideApp| {
            let st = a.session.doc().unwrap();
            st.doc.slides[st.selection.slide].shapes.iter().filter(|s| matches!(s.kind, deckcraft_model::ShapeKind::Table(_))).count()
        };
        assert!(!a.session.doc().unwrap().is_dirty());
        insert_table(&mut a, &rows(&[&["Name", "Qty"], &["{{loop items}}{{name}}", "{{qty}}{{endloop}}"]])).unwrap();
        assert!(a.session.doc().unwrap().is_dirty());
        assert_eq!(table(&a), 1);
        a.run("edit.undo", serde_json::json!({})).unwrap();
        assert_eq!(table(&a), 0);
    }

    #[test]
    fn a_tall_table_stays_inside_the_slide() {
        let mut a = app();
        insert_table(&mut a, &vec![vec!["x".to_string()]; MAX_ROWS]).unwrap();
        let st = a.session.doc().unwrap();
        let sh = st.doc.slides[st.selection.slide].shapes.iter().find(|s| matches!(s.kind, deckcraft_model::ShapeKind::Table(_))).unwrap();
        let (x, size) = (sh.xfrm.unwrap(), st.doc.slide_size);
        assert!(x.y >= 0.0 && x.y + x.h <= size.height + 0.5, "{x:?}");
    }

    #[test]
    fn bad_requests_are_refused_without_touching_the_presentation() {
        let mut a = app();
        let too_wide = vec![vec!["x".to_string(); MAX_COLS + 1]];
        let too_tall = vec![vec!["x".to_string()]; MAX_ROWS + 1];
        let long_cell = vec![vec!["x".repeat(MAX_CELL_CHARS + 1)]];
        for bad in [vec![], vec![vec![]], too_wide, too_tall, long_cell] {
            assert!(insert_table(&mut a, &bad).is_err());
        }
        assert!(!a.session.doc().unwrap().is_dirty());
    }

    #[test]
    fn line_breaks_in_a_cell_become_spaces() {
        let data = table_data(&rows(&[&["a\nb\tc"]])).unwrap();
        assert_eq!(data[0][0], "a b c");
    }
}
