import sqlite3

def reset_status():
    try:
        conn = sqlite3.connect('unity_api_nodes.db')
        cursor = conn.cursor()
        
        # 1. 'Symbol'로 분류되었거나, URL이 있는데 멤버가 수집되지 않은(is_scraped=1이지만 부실한) 항목들 리셋
        # 네임스페이스가 아닌데(url이 존재함) kind가 Symbol이거나 null인 것들 모조리 리셋
        cursor.execute("""
            UPDATE symbols 
            SET is_scraped = 0, kind = 'Symbol' 
            WHERE url IS NOT NULL 
              AND (kind = 'Symbol' OR kind IS NULL OR is_scraped = 1)
        """)
        
        count = cursor.rowcount
        conn.commit()
        print(f"Successfully reset {count} items for full re-scraping.")
        
        # 2. 멤버 데이터 삭제 (중복 방지를 위해 부모 페이지를 다시 긁을 때 멤버들도 다시 생성됨)
        # parent_id가 존재하고, 그 부모가 Class/Struct 등으로 분류될 것인 자식 노드들 삭제
        # 간단하게 URL이 없는(문서상 리스트 항목들) 자식 노드들 삭제
        cursor.execute("DELETE FROM symbols WHERE parent_id IS NOT NULL AND url IS NULL")
        print(f"Cleared {cursor.rowcount} member entries to prevent duplicates.")
        
        conn.commit()
        conn.close()
    except Exception as e:
        print(f"Reset Error: {e}")

if __name__ == "__main__":
    reset_status()
