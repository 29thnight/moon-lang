import sqlite3

def check_db():
    conn = sqlite3.connect('unity_api_nodes.db')
    cursor = conn.cursor()
    
    print("\n--- [1] 전체 심볼 개수 ---")
    cursor.execute("SELECT count(*) FROM symbols")
    print(f"Total: {cursor.fetchone()[0]}")
    
    print("\n--- [2] 최상위 네임스페이스 (Root Nodes) ---")
    cursor.execute("SELECT id, name FROM symbols WHERE parent_id = (SELECT id FROM symbols WHERE name = 'toc') LIMIT 5")
    roots = cursor.fetchall()
    for rid, rname in roots:
        print(f"ID {rid}: {rname}")
        
    print("\n--- [3] GameObject 클래스 노드 확인 ---")
    cursor.execute("SELECT id, full_name, kind, summary FROM symbols WHERE name = 'GameObject' AND kind = 'Class' LIMIT 1")
    row = cursor.fetchone()
    if row:
        sid, fname, kind, summary = row
        print(f"Symbol: {fname} (ID: {sid})")
        print(f"Kind: {kind}")
        print(f"Summary: {summary[:100]}...")
        
        print(f"\n--- [4] {fname}의 자식 노드 (Members) ---")
        cursor.execute("SELECT name, kind, summary FROM symbols WHERE parent_id = ? LIMIT 10", (sid,))
        members = cursor.fetchall()
        for mname, mkind, msummary in members:
            print(f"  - {mname} ({mkind}): {msummary[:50]}...")
    else:
        print("GameObject 클래스를 아직 수집하지 못했습니다. 수집 순서에 따라 시간이 걸릴 수 있습니다.")
        
    conn.close()

if __name__ == "__main__":
    check_db()
