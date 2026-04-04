import sqlite3
import os
import threading

class DBManager:
    def __init__(self, db_path="unity_api_nodes.db"):
        self.db_path = db_path
        self.conn = sqlite3.connect(self.db_path, check_same_thread=False)
        self.cursor = self.conn.cursor()
        self.lock = threading.Lock()
        
        # 순서 조정: 테이블 생성 -> 마이그레이션 -> 그 후 인덱스 생성
        self.init_db()
        self.migrate()
        self.create_indexes()
        self.enable_wal()

    def init_db(self):
        with self.lock:
            self.cursor.execute("""
                CREATE TABLE IF NOT EXISTS symbols (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    parent_id INTEGER,
                    name TEXT NOT NULL,
                    full_name TEXT,
                    kind TEXT,
                    summary TEXT,
                    signature TEXT,
                    url TEXT,
                    version TEXT,
                    is_scraped BOOLEAN DEFAULT 0,
                    FOREIGN KEY (parent_id) REFERENCES symbols(id)
                )
            """)
            self.conn.commit()

    def migrate(self):
        """기존 DB 구조를 최신 상태로 업데이트"""
        with self.lock:
            # 1. version 컬럼 추가 확인
            try:
                self.cursor.execute("SELECT version FROM symbols LIMIT 1")
            except sqlite3.OperationalError:
                print("Migrating: Adding 'version' column...")
                self.cursor.execute("ALTER TABLE symbols ADD COLUMN version TEXT DEFAULT '6000.4'")
                self.conn.commit()
            
            # 2. signature 컬럼 추가 확인
            try:
                self.cursor.execute("SELECT signature FROM symbols LIMIT 1")
            except sqlite3.OperationalError:
                print("Migrating: Adding 'signature' column...")
                self.cursor.execute("ALTER TABLE symbols ADD COLUMN signature TEXT")
                self.conn.commit()

    def create_indexes(self):
        """컬럼이 모두 존재하는 것이 확실해진 후 인덱스 생성"""
        with self.lock:
            self.cursor.execute("CREATE INDEX IF NOT EXISTS idx_parent ON symbols(parent_id)")
            self.cursor.execute("CREATE INDEX IF NOT EXISTS idx_full_name ON symbols(full_name)")
            self.cursor.execute("CREATE INDEX IF NOT EXISTS idx_version ON symbols(version)")
            self.cursor.execute("CREATE INDEX IF NOT EXISTS idx_is_scraped ON symbols(is_scraped)")
            self.cursor.execute("CREATE INDEX IF NOT EXISTS idx_url ON symbols(url)")
            self.conn.commit()

    def enable_wal(self):
        with self.lock:
            self.cursor.execute("PRAGMA journal_mode=WAL")
            self.conn.commit()

    def insert_symbol(self, data):
        with self.lock:
            try:
                is_scraped = data.get('is_scraped', 0)
                self.cursor.execute("""
                    INSERT INTO symbols (parent_id, name, full_name, kind, summary, signature, url, version, is_scraped)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                """, (
                    data.get('parent_id'),
                    data['name'],
                    data.get('full_name'),
                    data.get('kind'),
                    data.get('summary'),
                    data.get('signature'),
                    data.get('url'),
                    data.get('version'),
                    is_scraped
                ))
                self.conn.commit()
                return self.cursor.lastrowid
            except Exception as e:
                print(f"Error inserting {data['name']}: {e}")
                return None

    def update_detail(self, symbol_id, kind=None, summary=None, signature=None):
        with self.lock:
            fields = []
            params = []
            if kind:
                fields.append("kind = ?")
                params.append(kind)
            if summary:
                fields.append("summary = ?")
                params.append(summary)
            if signature:
                fields.append("signature = ?")
                params.append(signature)
            
            if not fields:
                # 단순히 scraped 상태만 업데이트할 경우
                self.cursor.execute("UPDATE symbols SET is_scraped = 1 WHERE id = ?", (symbol_id,))
            else:
                fields.append("is_scraped = 1")
                params.append(symbol_id)
                query = f"UPDATE symbols SET {', '.join(fields)} WHERE id = ?"
                self.cursor.execute(query, params)
            self.conn.commit()

    def exists(self, url, version):
        with self.lock:
            self.cursor.execute("SELECT id, is_scraped FROM symbols WHERE url = ? AND version = ?", (url, version))
            return self.cursor.fetchone()

    def find_node(self, name, parent_id, version):
        with self.lock:
            self.cursor.execute("SELECT id, is_scraped FROM symbols WHERE name = ? AND parent_id IS ? AND version = ?", 
                             (name, parent_id, version))
            return self.cursor.fetchone()

    def close(self):
        with self.lock:
            if self.conn:
                self.conn.close()
