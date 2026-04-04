import sqlite3

def check_db():
    try:
        conn = sqlite3.connect('unity_api_nodes.db')
        cursor = conn.cursor()
        
        print("--- Symbols with kind='Symbol' (Limit 10) ---")
        cursor.execute("SELECT id, name, kind, is_scraped FROM symbols WHERE kind='Symbol' LIMIT 10")
        for row in cursor.fetchall():
            print(row)
            
        print("\n--- Symbols with refined kinds (Limit 10) ---")
        cursor.execute("SELECT id, name, kind, is_scraped FROM symbols WHERE kind NOT IN ('Namespace', 'Symbol') LIMIT 10")
        for row in cursor.fetchall():
            print(row)
            
        print("\n--- Summary Counts ---")
        cursor.execute("SELECT kind, count(*) FROM symbols GROUP BY kind")
        for row in cursor.fetchall():
            print(f"{row[0]}: {row[1]}")
            
        conn.close()
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    check_db()
