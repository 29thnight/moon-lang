# Unity API Scraper (Intellisense-ready)

This toolkit fetches Unity 6.0 (6000.4) Scripting API documentation and stores it in a structured SQLite database.

## Features
- **Comprehensive**: Includes `UnityEngine`, `UnityEditor`, and all sub-namespaces.
- **Intellisense-level**: Stores Classes, Structs, Enums, Interfaces, and their members (Properties, Methods, etc.) with summaries.
- **Resumable**: If the script stops, it will pick up from where it left off.
- **Rate-limited**: Avoids over-taxing Unity's documentation servers.

## Files
- `toc_parser.py`: Parses the Table of Contents from Unity docs.
- `db_manager.py`: Manages the SQLite database schema.
- `page_parser.py`: Extracts information from HTML pages.
- `main.py`: Main orchestration script.

## How to use
1. Run the main scraper:
   ```bash
   python unity_docs_scraper/main.py
   ```
2. The data will be stored in `unity_api_6000.db`.

## Note on Signatures
Unity's summary tables do not contain the full method signatures (parameters). To obtain true Intellisense-level parameters, the scraper would need to visit each of the ~100k member pages, which can take a very long time and may result in a timeout or IP block. 

The current version collects **Member Name** and **Summary**, which is often enough for search and basic Intellisense lookup. 
