-- 跑马灯(首页顶部滚动横幅):内容支持 Markdown/HTML,每条可自定义文字颜色
CREATE TABLE IF NOT EXISTS marquees (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL DEFAULT '',
    color TEXT NOT NULL DEFAULT '#ef4444',
    enabled INTEGER NOT NULL DEFAULT 1,
    sort INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
