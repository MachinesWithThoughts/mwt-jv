use crate::{
    build_info,
    document::{Document, TableData, TreeRow, visible_rows},
    highlight::{self, KEY, MUTED, TEXT},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

const BG: Color = Color::Rgb(15, 22, 34);
const SELECTED: Color = Color::Rgb(35, 52, 75);
const ACCENT: Color = Color::Rgb(91, 211, 203);
// Explicit text presentation keeps these controls one terminal cell wide.
// Emoji presentation can disagree with the terminal's cursor width and shift
// the JSON text when a highlighted row is repainted.
const FOLDED: &str = "▶\u{fe0e}  ";
const OPEN: &str = "▼\u{fe0e}  ";
const LEAF: &str = "   ";

pub struct App {
    pub doc: Document,
    pub source: String,
    pub collapsed: HashSet<usize>,
    pub rows: Vec<TreeRow>,
    pub selected: usize,
    pub top: usize,
    pub horizontal: u16,
    pub table: Option<TableData>,
    pub table_selected: usize,
    pub table_top: usize,
    pub help: bool,
    body: Rect,
    gutter: u16,
    page: usize,
}

impl App {
    pub fn new(doc: Document, source: String) -> Self {
        // Show the top levels immediately; expand deeper branches on demand.
        let collapsed = doc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.depth >= 2 && !n.children.is_empty())
            .map(|(id, _)| id)
            .collect();
        let rows = visible_rows(&doc, &collapsed);
        Self {
            doc,
            source,
            collapsed,
            rows,
            selected: 0,
            top: 0,
            horizontal: 0,
            table: None,
            table_selected: 0,
            table_top: 0,
            help: false,
            body: Rect::default(),
            gutter: 0,
            page: 1,
        }
    }

    pub fn current_id(&self) -> usize {
        self.rows[self.selected].id
    }

    fn rebuild(&mut self, id: usize) {
        self.rows = visible_rows(&self.doc, &self.collapsed);
        self.selected = self
            .rows
            .iter()
            .position(|r| r.id == id && !r.closing)
            .unwrap_or(0);
    }

    fn toggle(&mut self) {
        let id = self.current_id();
        if !self.doc.nodes[id].children.is_empty() {
            if !self.collapsed.remove(&id) {
                self.collapsed.insert(id);
            }
            self.rebuild(id);
        }
    }

    fn move_by(&mut self, delta: isize) {
        if let Some(table) = &self.table {
            self.table_selected = self
                .table_selected
                .saturating_add_signed(delta)
                .min(table.rows.len().saturating_sub(1));
        } else {
            self.selected = self
                .selected
                .saturating_add_signed(delta)
                .min(self.rows.len().saturating_sub(1));
        }
    }

    pub fn key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('q' | 'c'))
        {
            return true;
        }
        if self.help {
            self.help = false;
            return false;
        }
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            match key.code {
                KeyCode::Right => {
                    self.horizontal = self.horizontal.saturating_add(8);
                    return false;
                }
                KeyCode::Left => {
                    self.horizontal = self.horizontal.saturating_sub(8);
                    return false;
                }
                _ => {}
            }
        }
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('?') | KeyCode::F(1) => self.help = true,
            KeyCode::Up | KeyCode::Char('k') => self.move_by(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_by(1),
            KeyCode::PageDown | KeyCode::Char('f') => self.move_by(self.page as isize),
            KeyCode::PageUp | KeyCode::Char('b') => self.move_by(-(self.page as isize)),
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected = 0;
                self.table_selected = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                if let Some(table) = &self.table {
                    self.table_selected = table.rows.len().saturating_sub(1);
                } else {
                    self.selected = self.rows.len() - 1;
                }
            }
            KeyCode::Char('t') => {
                if self.table.is_some() {
                    self.table = None;
                } else {
                    self.table = Some(TableData::new(&self.doc, self.current_id()));
                    self.table_selected = 0;
                    self.table_top = 0;
                }
                self.horizontal = 0;
            }
            KeyCode::Esc => {
                self.table = None;
                self.horizontal = 0;
            }
            KeyCode::Enter if self.table.is_some() => {
                if let Some(id) = self
                    .table
                    .as_ref()
                    .and_then(|t| t.node_ids.get(self.table_selected))
                    .copied()
                {
                    let mut parent = self.doc.nodes[id].parent;
                    while let Some(p) = parent {
                        self.collapsed.remove(&p);
                        parent = self.doc.nodes[p].parent;
                    }
                    self.table = None;
                    self.horizontal = 0;
                    self.rebuild(id);
                }
            }
            KeyCode::Left | KeyCode::Char('h') if self.table.is_some() => {
                self.horizontal = self.horizontal.saturating_sub(8)
            }
            KeyCode::Right | KeyCode::Char('l') if self.table.is_some() => {
                self.horizontal = self.horizontal.saturating_add(8)
            }
            KeyCode::Enter | KeyCode::Char(' ') if self.table.is_none() => self.toggle(),
            KeyCode::Right | KeyCode::Char('l') => {
                let id = self.current_id();
                if self.collapsed.remove(&id) {
                    self.rebuild(id);
                } else if !self.doc.nodes[id].children.is_empty()
                    && !self.rows[self.selected].closing
                {
                    self.selected += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let id = self.current_id();
                if !self.doc.nodes[id].children.is_empty() && !self.collapsed.contains(&id) {
                    self.toggle();
                } else if let Some(parent) = self.doc.nodes[id].parent {
                    self.rebuild(parent);
                }
            }
            KeyCode::Char('e') if self.table.is_none() => {
                let id = self.current_id();
                self.collapsed.clear();
                self.rebuild(id);
            }
            KeyCode::Char('c') if self.table.is_none() => {
                self.collapsed = self
                    .doc
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| !n.children.is_empty())
                    .map(|(id, _)| id)
                    .collect();
                self.rebuild(0);
            }
            KeyCode::Char('0') => self.horizontal = 0,
            _ => {}
        }
        false
    }

    pub fn mouse(&mut self, mouse: MouseEvent) {
        if self.help {
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollDown => self.move_by(3),
            MouseEventKind::ScrollUp => self.move_by(-3),
            MouseEventKind::Down(MouseButton::Left)
                if self.body.contains((mouse.column, mouse.row).into()) =>
            {
                let row = (mouse.row - self.body.y) as usize;
                if let Some(table) = &self.table {
                    if row >= 2 && row - 2 + self.table_top < table.rows.len() {
                        self.table_selected = row - 2 + self.table_top;
                    }
                } else if row + self.top < self.rows.len() {
                    self.selected = row + self.top;
                    let node = &self.doc.nodes[self.current_id()];
                    let triangle_x = self.body.x + self.gutter + (node.depth * 2) as u16;
                    if self.horizontal == 0
                        && mouse.column >= triangle_x
                        && mouse.column < triangle_x + 2
                    {
                        self.toggle();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(
            Block::default().style(Style::default().bg(BG).fg(TEXT)),
            area,
        );
        let parts = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
        let mode = if self.table.is_some() {
            "TABLE"
        } else {
            "TREE"
        };
        let header = Line::from(vec![
            Span::styled(" ◈ MWT JSON VIEWER ", Style::default().fg(ACCENT).bold()),
            Span::styled(format!(" {mode} "), Style::default().bg(SELECTED).fg(KEY)),
            Span::styled(format!("  {}", self.source.escape_debug()), MUTED),
        ]);
        frame.render_widget(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(SELECTED)),
            parts[0],
        );
        let build = build_info::header(area.width.saturating_sub(26) as usize);
        let build_width = (build.width() + 1).min(area.width as usize) as u16;
        let top_line = Layout::horizontal([Constraint::Min(0), Constraint::Length(build_width)])
            .split(Rect::new(
                parts[0].x,
                parts[0].y,
                parts[0].width,
                parts[0].height.min(1),
            ));
        frame.render_widget(Paragraph::new(header), top_line[0]);
        frame.render_widget(
            Paragraph::new(format!("{build} "))
                .right_aligned()
                .style(Style::default().fg(MUTED)),
            top_line[1],
        );
        let id = self.current_id();
        let pointer = &self.doc.nodes[id].pointer;
        let path = if pointer.is_empty() {
            "/ (root)"
        } else {
            pointer
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(
                        "{:>width$} {:>4} │ ",
                        "LINE",
                        "DEP",
                        width = self.doc.lines.to_string().len().max(4)
                    ),
                    MUTED,
                ),
                Span::styled(path.escape_debug().to_string(), ACCENT),
            ])),
            parts[1],
        );
        self.body = parts[2];
        self.gutter = (self.doc.lines.to_string().len().max(4) + 8) as u16;
        self.gutter = self.gutter.min(self.body.width);
        if self.table.is_some() {
            self.draw_table(frame);
        } else {
            self.draw_tree(frame);
        }
        let node = &self.doc.nodes[id];
        let position = if let Some(t) = &self.table {
            format!(
                "{} / {} rows",
                if t.rows.is_empty() {
                    0
                } else {
                    self.table_selected + 1
                },
                t.rows.len()
            )
        } else {
            format!("{} / {} visible", self.selected + 1, self.rows.len())
        };
        let status = format!(
            " {position}  ·  line {} / {}  ·  depth {}  ·  {} nodes  ·  x:{}",
            node.line,
            self.doc.lines,
            node.depth,
            self.doc.nodes.len(),
            self.horizontal
        );
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(MUTED).bg(SELECTED)),
            parts[3],
        );
        frame.render_widget(
            Paragraph::new(if self.table.is_some() {
                " ↑↓ move   ←→ scroll   Enter inspect row   t/Esc tree   ? help   ^Q quit"
            } else {
                " ↑↓ move   ◀/▶ fold   Space toggle   t table   Shift+←→ scroll   ? help   ^Q quit"
            })
            .style(Style::default().fg(ACCENT)),
            parts[4],
        );
        if self.help {
            self.draw_help(frame);
        }
    }

    fn keep_visible(selected: usize, top: &mut usize, height: usize) {
        if selected < *top {
            *top = selected;
        }
        if selected >= top.saturating_add(height) {
            *top = selected.saturating_sub(height.saturating_sub(1));
        }
    }

    fn gutter_line(&self, line: usize, depth: usize) -> Line<'static> {
        Line::from(Span::styled(
            format!(
                "{:>width$} {:>4} │ ",
                line,
                depth,
                width = self.doc.lines.to_string().len().max(4)
            ),
            MUTED,
        ))
    }

    fn draw_tree(&mut self, frame: &mut Frame) {
        self.page = (self.body.height as usize).max(1);
        Self::keep_visible(self.selected, &mut self.top, self.page);
        for (screen_row, row) in self.rows.iter().skip(self.top).take(self.page).enumerate() {
            let node = &self.doc.nodes[row.id];
            let value = self.doc.value(row.id);
            let y = self.body.y + screen_row as u16;
            let area = Rect::new(self.body.x, y, self.body.width, 1);
            let selected = self.top + screen_row == self.selected;
            if selected {
                frame.render_widget(Block::default().style(Style::default().bg(SELECTED)), area);
            }
            frame.render_widget(
                Paragraph::new(self.gutter_line(
                    if row.closing {
                        node.end_line
                    } else {
                        node.line
                    },
                    node.depth,
                )),
                Rect::new(area.x, y, self.gutter, 1),
            );
            let mut spans = vec![Span::raw("  ".repeat(node.depth))];
            let expandable = !node.children.is_empty();
            let collapsed = self.collapsed.contains(&row.id);
            spans.push(Span::styled(
                if expandable && !row.closing {
                    if collapsed { FOLDED } else { OPEN }
                } else {
                    LEAF
                },
                ACCENT,
            ));
            if !row.closing
                && let Some(key) = &node.key
            {
                spans.push(Span::styled(serde_json::to_string(key).unwrap(), KEY));
                spans.push(Span::raw(": "));
            }
            let (open, close) = if value.is_object() {
                ("{", "}")
            } else {
                ("[", "]")
            };
            let text = if row.closing {
                close.to_string()
            } else if expandable {
                if collapsed {
                    format!("{open} … {close}")
                } else {
                    open.to_string()
                }
            } else {
                value.to_string()
            };
            spans.extend(highlight::json(&text).spans);
            if node.comma && (row.closing || collapsed || !expandable) {
                spans.push(Span::raw(","));
            }
            if expandable && collapsed {
                spans.push(Span::styled(
                    format!(
                        "  {} {}",
                        node.children.len(),
                        if value.is_array() { "items" } else { "keys" }
                    ),
                    MUTED,
                ));
            }
            frame.render_widget(
                Paragraph::new(Line::from(spans)).scroll((0, self.horizontal)),
                Rect::new(
                    area.x + self.gutter,
                    y,
                    area.width.saturating_sub(self.gutter),
                    1,
                ),
            );
        }
    }

    fn draw_table(&mut self, frame: &mut Frame) {
        let table = self.table.as_ref().unwrap();
        self.page = self.body.height.saturating_sub(2).max(1) as usize;
        Self::keep_visible(self.table_selected, &mut self.table_top, self.page);
        // Keep full values; horizontal scrolling reveals even very long raw JSON cells.
        let widths: Vec<usize> = table
            .headers
            .iter()
            .enumerate()
            .map(|(col, header)| {
                table
                    .rows
                    .iter()
                    .map(|r| r[col].width())
                    .max()
                    .unwrap_or(0)
                    .max(header.escape_debug().to_string().width())
                    .max(5)
            })
            .collect();
        let content_x = self.body.x + self.gutter;
        let content_width = self.body.width.saturating_sub(self.gutter);
        let mut header = vec![];
        let mut divider = String::new();
        for (name, width) in table.headers.iter().zip(&widths) {
            let name = name.escape_debug().to_string();
            header.push(Span::styled(
                format!(
                    "{name}{} │ ",
                    " ".repeat(width.saturating_sub(name.width()))
                ),
                Style::default().fg(ACCENT).bold(),
            ));
            divider.push_str(&format!("{}─┼─", "─".repeat(*width)));
        }
        if self.body.height > 0 {
            frame.render_widget(
                Paragraph::new("LINE  DEP │").style(Style::default().fg(MUTED)),
                Rect::new(self.body.x, self.body.y, self.gutter, 1),
            );
            frame.render_widget(
                Paragraph::new(Line::from(header)).scroll((0, self.horizontal)),
                Rect::new(content_x, self.body.y, content_width, 1),
            );
        }
        if self.body.height > 1 {
            frame.render_widget(
                Paragraph::new(divider)
                    .style(Style::default().fg(SELECTED))
                    .scroll((0, self.horizontal)),
                Rect::new(content_x, self.body.y + 1, content_width, 1),
            );
        }
        if table.rows.is_empty() && self.body.height > 2 {
            frame.render_widget(
                Paragraph::new(" Empty container · t returns to tree")
                    .style(Style::default().fg(MUTED)),
                Rect::new(self.body.x, self.body.y + 2, self.body.width, 1),
            );
        }
        for (screen_row, cells) in table
            .rows
            .iter()
            .skip(self.table_top)
            .take(self.body.height.saturating_sub(2) as usize)
            .enumerate()
        {
            let index = self.table_top + screen_row;
            let node = &self.doc.nodes[table.node_ids[index]];
            let y = self.body.y + 2 + screen_row as u16;
            if index == self.table_selected {
                frame.render_widget(
                    Block::default().style(Style::default().bg(SELECTED)),
                    Rect::new(self.body.x, y, self.body.width, 1),
                );
            }
            frame.render_widget(
                Paragraph::new(self.gutter_line(node.line, node.depth)),
                Rect::new(self.body.x, y, self.gutter, 1),
            );
            let mut spans = vec![];
            for (cell, width) in cells.iter().zip(&widths) {
                spans.extend(highlight::json(cell).spans);
                spans.push(Span::styled(
                    format!("{} │ ", " ".repeat(width.saturating_sub(cell.width()))),
                    MUTED,
                ));
            }
            frame.render_widget(
                Paragraph::new(Line::from(spans)).scroll((0, self.horizontal)),
                Rect::new(content_x, y, content_width, 1),
            );
        }
    }

    fn draw_help(&self, frame: &mut Frame) {
        let area = frame.area();
        let width = 72.min(area.width);
        let height = 19.min(area.height);
        let popup = Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        );
        frame.render_widget(Clear, popup);
        let text = vec![
            "↑/↓, j/k       Move selection",
            "←/→, h/l       Collapse / expand; parent / first child",
            "Space, Enter   Toggle selected container",
            "Mouse          Click triangle to fold; wheel to move",
            "t              Toggle table for selected node",
            "Enter (table)  Inspect selected row in the tree",
            "←/→ (table)    Scroll horizontally",
            "Shift+←/→      Scroll horizontally in either mode",
            "0              Reset horizontal scroll",
            "PgUp/PgDn, b/f Page up / down",
            "Home/End, g/G  First / last row",
            "e / c          Expand / collapse all (tree)",
            "Esc            Return to tree / reset scroll",
            "Ctrl+Q, q      Quit (Ctrl+C also works)",
            "",
            "Lines refer to formatted JSON; root depth is 0.",
            "Press any key to close help.",
        ];
        frame.render_widget(
            Paragraph::new(text.into_iter().map(Line::from).collect::<Vec<_>>())
                .style(Style::default().fg(TEXT).bg(BG))
                .block(
                    Block::default()
                        .title(" Keyboard & mouse ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(ACCENT)),
                ),
            popup,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use serde_json::json;
    fn press(app: &mut App, key: KeyCode) {
        app.key(KeyEvent::new(key, KeyModifiers::NONE));
    }

    #[test]
    fn keyboard_fold_table_and_inspect() {
        let mut app = App::new(
            Document::new(json!({"users": [{"name": "Ada"}, {"name": "Grace"}]})),
            "test".into(),
        );
        press(&mut app, KeyCode::Down);
        let users = app.current_id();
        press(&mut app, KeyCode::Char(' '));
        assert!(app.collapsed.contains(&users));
        press(&mut app, KeyCode::Char('t'));
        assert_eq!(app.table.as_ref().unwrap().headers, ["name"]);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert!(app.table.is_none());
        assert_eq!(app.doc.value(app.current_id()), &json!({"name": "Grace"}));
        assert!(!app.collapsed.contains(&users));
    }

    #[test]
    fn ctrl_q_quits_from_tree_table_and_help_and_escape_returns_to_tree() {
        let mut app = App::new(Document::new(json!({"items": [1, 2]})), "test".into());
        let quit = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert!(app.key(quit));
        press(&mut app, KeyCode::Down);
        let selected = app.current_id();
        press(&mut app, KeyCode::Char('t'));
        assert!(app.key(quit));
        app.horizontal = 8;
        assert!(!app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(app.table.is_none());
        assert_eq!(app.current_id(), selected);
        assert_eq!(app.horizontal, 0);
        press(&mut app, KeyCode::Char('?'));
        assert!(app.help);
        assert!(app.key(quit));
    }

    #[test]
    fn renders_tree_table_help_and_tiny_terminals() {
        let mut app = App::new(
            Document::new(json!({"users": [{"name": "雪", "data": {"x": 1}}]})),
            "fixture.json".into(),
        );
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        let screen: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(screen.contains("MWT JSON VIEWER"));
        assert!(screen.contains("fixture.json"));
        assert!(screen.contains("depth 0"));
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char('t'));
        terminal.draw(|f| app.draw(f)).unwrap();
        let screen: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(screen.contains("TABLE"));
        assert!(screen.contains("{\"x\":1}"));
        for (w, h) in [(1, 1), (10, 3), (40, 10)] {
            terminal.resize(Rect::new(0, 0, w, h)).unwrap();
            terminal.draw(|f| app.draw(f)).unwrap();
            app.help = true;
            terminal.draw(|f| app.draw(f)).unwrap();
            app.help = false;
        }
    }

    #[test]
    fn mouse_triangle_toggles_root() {
        let mut app = App::new(Document::new(json!([1, 2])), "test".into());
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        app.mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: app.body.x + app.gutter,
            row: app.body.y,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.rows.len(), 1);
    }

    #[test]
    fn build_information_stays_visible_beside_a_long_filename_in_both_modes() {
        let mut app = App::new(Document::new(json!([1, 2])), "a".repeat(300));
        let mut terminal = Terminal::new(TestBackend::new(140, 25)).unwrap();
        for _ in 0..2 {
            terminal.draw(|f| app.draw(f)).unwrap();
            let buffer = terminal.backend().buffer();
            let top: String = (0..140).map(|x| buffer[(x, 0)].symbol()).collect();
            assert!(top.starts_with(" ◈ MWT JSON VIEWER "));
            assert!(top.contains(&format!(
                "v{} · built {} (",
                build_info::VERSION,
                build_info::TIMESTAMP
            )));
            assert!(top.ends_with(") "));
            press(&mut app, KeyCode::Char('t'));
        }
    }

    #[test]
    fn navigating_changes_highlight_without_changing_row_text() {
        let mut app = App::new(
            Document::new(json!({"users": [{"name": "雪"}], "enabled": true})),
            "test".into(),
        );
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        let original = terminal.backend().buffer().clone();
        let rows = app.rows.clone();
        for key in [KeyCode::Down, KeyCode::Down, KeyCode::Down, KeyCode::Up] {
            let previous_selection = app.selected;
            press(&mut app, key);
            terminal.draw(|f| app.draw(f)).unwrap();
            let current = terminal.backend().buffer();
            for y in app.body.y..app.body.bottom() {
                for x in app.body.x..app.body.right() {
                    assert_eq!(
                        original[(x, y)].symbol(),
                        current[(x, y)].symbol(),
                        "Navigation changed row content at ({x}, {y})"
                    );
                }
            }
            assert_eq!(app.rows, rows);
            assert_ne!(app.selected, previous_selection);
            assert_eq!(current[(0, app.body.y + app.selected as u16)].bg, SELECTED);
            assert_eq!(current[(0, app.body.y + previous_selection as u16)].bg, BG);
        }
        // Container and leaf controls occupy exactly the same number of cells.
        for marker in [FOLDED, OPEN, LEAF] {
            assert_eq!(marker.width(), 3);
            assert!(!marker.contains('\u{fe0f}'));
        }
    }
}
