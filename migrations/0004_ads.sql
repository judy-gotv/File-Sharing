-- 弹窗公告
CREATE TABLE IF NOT EXISTS announcements(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL DEFAULT '',
  content TEXT NOT NULL DEFAULT '',
  enabled INTEGER NOT NULL DEFAULT 1,
  sort INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
-- 页脚内容(原始 Markdown/HTML,展示时渲染)
INSERT OR IGNORE INTO settings(key, value) VALUES('footer_html', '');
-- 要求(必须/可选) -> 置顶(可自定义颜色)
ALTER TABLE items ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN pin_color TEXT NOT NULL DEFAULT '#f59e0b';
UPDATE items SET pinned = required;
ALTER TABLE items DROP COLUMN required;
