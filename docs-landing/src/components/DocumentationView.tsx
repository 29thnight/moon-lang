import { useState, useEffect } from 'react'

interface DocRoute {
  id: string
  title: string
  path: string
}

interface DocGroup {
  id: string
  title: string
  items: DocRoute[]
}

const NAV_PATHS: Record<string, string> = {
  en: 'docs/en/_nav.json',
  ko: 'docs/ko/_nav.json',
}

function useDocGroups(lang: 'en' | 'ko'): DocGroup[] {
  const [groups, setGroups] = useState<DocGroup[]>([])

  useEffect(() => {
    const navPath = NAV_PATHS[lang]
    fetch(import.meta.env.BASE_URL + navPath)
      .then(res => {
        if (!res.ok) throw new Error(`Failed to load ${navPath}`)
        return res.json() as Promise<DocGroup[]>
      })
      .then(setGroups)
      .catch(err => {
        console.warn('Navigation load failed, using empty nav:', err)
        setGroups([])
      })
  }, [lang])

  return groups
}

interface DocumentationViewProps {
  initialDocId?: string
  lang: 'en' | 'ko'
}

const highlightCode = (code: string, lang: string) => {
  const l = lang.toLowerCase();
  
  const isInsideSpan = (html: string, offset: number) => {
    const open = html.lastIndexOf('<span', offset);
    const close = html.lastIndexOf('</span>', offset);
    return open > close;
  };

  const highlightPrsm = (c: string) => {
    let h = c;
    const knownTypesStr = "\\b(Int|Float|Double|Bool|String|Long|Byte|Unit|Vector2|Vector3|Vector4|Quaternion|Matrix4x4|Color|Color32|Rect|Bounds|Ray|RaycastHit|LayerMask|MonoBehaviour|ScriptableObject|GameObject|Transform|Component|Rigidbody|Rigidbody2D|Collider|Collider2D|BoxCollider|SphereCollider|CapsuleCollider|MeshCollider|CharacterController|Collision|Collision2D|Camera|Light|Renderer|MeshRenderer|SkinnedMeshRenderer|SpriteRenderer|MeshFilter|Material|Shader|Texture|Texture2D|Sprite|RenderTexture|Animator|Animation|AnimationClip|AudioSource|AudioClip|AudioListener|Canvas|CanvasGroup|RectTransform|Image|Text|Button|Slider|Toggle|InputField|Dropdown|ScrollRect|TextMeshPro|TextMeshProUGUI|TMP_Text|ParticleSystem|TrailRenderer|LineRenderer|NavMeshAgent|Terrain|Tilemap|Input|Time|Debug|Mathf|Physics|Physics2D|Application|SceneManager|Resources|PlayerPrefs|Screen|Cursor|Gizmos|UnityEvent|UnityAction|NativeArray|List|Dictionary|HashSet|Queue|Stack|Array)\\b";

    // 1. Comments & Strings
    h = h.replace(/(\/\/.*$)/gm, '<span class="prsm-comment">$1</span>');
    h = h.replace(/\/\*[\s\S]*?\*\//g, '<span class="prsm-comment">$&</span>');
    h = h.replace(/("[^"\\]*(?:\\[\s\S][^"\\]*)*")/g, (match, _, offset) => {
       if (isInsideSpan(h, offset)) return match;
       return `<span class="prsm-string">${match}</span>`;
    });

    // 2. Headings & Declarations (Specific context coloring)
    h = h.replace(/\b(using)\s+([a-zA-Z0-9_.]+)/g, (match, p1, p2, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-keyword">${p1}</span> <span class="prsm-default">${p2}</span>`;
    });

    h = h.replace(/\b(component|asset|class)\s+([A-Z][a-zA-Z0-9_]*)/g, (match, p1, p2, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-keyword">${p1}</span> <span class="prsm-type">${p2}</span>`;
    });

    h = h.replace(/\b(enum|data)\s+([A-Z][a-zA-Z0-9_]*)/g, (match, p1, p2, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-keyword">${p1}</span> <span class="prsm-default">${p2}</span>`;
    });

    // 3. Member Access (The dot rule - distinguished by Known Types)
    // For static access like EnemyState.Idle, EnemyState should be white if not a known type.
    h = h.replace(/\b([A-Z][a-zA-Z0-9_]*)\.([a-zA-Z0-9_]+)\b/g, (match, p1, p2, offset) => {
      if (isInsideSpan(h, offset)) return match;
      
      const isKnown = new RegExp(knownTypesStr).test(p1);
      const typeSpan = isKnown ? `<span class="prsm-type">${p1}</span>` : p1;
      
      const after = h.substring(offset + match.length).trim();
      const isFn = after.startsWith('(');
      
      // Known blue properties/statics
      const isBlue = isFn || p1 === 'Time' || p1 === 'Vector3' || p1 === 'Mathf';
      const cls = isBlue ? 'prsm-function' : 'prsm-property';

      return `${typeSpan}.<span class="${cls}">${p2}</span>`;
    });

    // 4. Type Annotations & Generics (Always Teal)
    h = h.replace(/:\s*([A-Z][a-zA-Z0-9_]*)/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      const name = match.split(':')[1].trim();
      return `:<span class="prsm-type">${name}</span>`;
    });

    h = h.replace(/<([A-Z][a-zA-Z0-9_]*)>/g, (match, p1, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `&lt;<span class="prsm-type">${p1}</span>&gt;`;
    });

    // 5. Base Keywords & Constants
    const keywords = /\b(serialize|require|optional|child|parent|public|private|protected|val|var|func|coroutine|override|return|intrinsic|if|else|when|for|while|in|until|downTo|step|break|continue|is|wait|start|stop|stopAll|listen|awake|update|fixedUpdate|lateUpdate|onEnable|onDisable|onDestroy|onTriggerEnter|onTriggerExit|onTriggerStay|onCollisionEnter|onCollisionExit|onCollisionStay)\b/g;
    h = h.replace(keywords, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-keyword">${match}</span>`;
    });

    const builtins = /\b(true|false|null|nextFrame|fixedFrame)\b/g;
    h = h.replace(builtins, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-builtin">${match}</span>`;
    });

    h = h.replace(/\b(this|transform|gameObject)\b/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-function">${match}</span>`;
    });

    h = h.replace(/(=&gt;|=>)/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-operator-keyword">${match}</span>`;
    });

    // 6. Generic PascalCase fallback (Default to White/Foreground)
    h = h.replace(/\b([A-Z][a-zA-Z0-9_]*)\b/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      // Default PascalCase is white (Matches Islands Dark's DEFAULT_CLASS_REFERENCE)
      return `<span class="prsm-default">${match}</span>`;
    });

    // 7. Functions & Properties
    h = h.replace(/\b([a-z][a-zA-Z0-9_]*)(?=\s*\()/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-function">${match}</span>`;
    });

    h = h.replace(/\.([a-z][a-zA-Z0-9_]*)\b/g, (match, p1, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `.<span class="prsm-property">${p1}</span>`;
    });

    // 8. Numbers & Annotations
    h = h.replace(/(@[a-zA-Z_][a-zA-Z0-9_]*)/g, '<span class="prsm-annotation">$1</span>');
    h = h.replace(/\b(\d+(\.\d+)?[sf]?)\b/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-number">${match}</span>`;
    });

    return h;
  };

  const highlightCSharp = (c: string) => {
    let h = c;

    // Comments
    h = h.replace(/(\/\/.*$)/gm, '<span class="prsm-comment">$1</span>');
    h = h.replace(/\/\*[\s\S]*?\*\//g, '<span class="prsm-comment">$&</span>');

    // Strings
    h = h.replace(/("[^"\\]*(?:\\[\s\S][^"\\]*)*")/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-string">${match}</span>`;
    });

    // Keywords
    const csKeywords = /\b(abstract|as|base|bool|break|byte|case|catch|char|checked|class|const|continue|decimal|default|delegate|do|double|else|enum|event|explicit|extern|false|finally|fixed|float|for|foreach|goto|if|implicit|in|int|interface|internal|is|lock|long|namespace|new|null|object|operator|out|override|params|private|protected|public|readonly|ref|return|sbyte|sealed|short|sizeof|stackalloc|static|string|struct|switch|this|throw|true|try|typeof|uint|ulong|unchecked|unsafe|ushort|using|var|virtual|void|volatile|while|yield)\b/g;
    h = h.replace(csKeywords, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-keyword">${match}</span>`;
    });

    // Types (PascalCase)
    h = h.replace(/\b([A-Z][a-zA-Z0-9_]*)\b/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-type">${match}</span>`;
    });

    // Methods (followed by parenthesis)
    h = h.replace(/\b([a-z][a-zA-Z0-9_]*)(?=\s*\()/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-function">${match}</span>`;
    });

    // Properties (after dot)
    h = h.replace(/\.([a-z][a-zA-Z0-9_]*)\b/g, (match, p1, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `.<span class="prsm-property">${p1}</span>`;
    });

    // Numbers
    h = h.replace(/\b(\d+(\.\d+)?[fFdDmM]?)\b/g, (match, _, _2, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-number">${match}</span>`;
    });

    // Attributes [...]
    h = h.replace(/(\[[A-Z][a-zA-Z0-9_]*(?:\([^)]*\))?\])/g, (match, _, offset) => {
      if (isInsideSpan(h, offset)) return match;
      return `<span class="prsm-annotation">${match}</span>`;
    });

    return h;
  };

  if (l === 'json') {
    return code
      .replace(/"([^"]+)":/g, '<span class="code-key">"$1"</span>:')
      .replace(/: \s*"([^"]*)"/g, ': <span class="code-string">"$1"</span>')
      .replace(/\b(true|false|null)\b/g, '<span class="code-keyword">$1</span>')
      .replace(/\b(\d+)\b/g, '<span class="code-keyword">$1</span>');
  }

  if (l === 'prsm' || l === 'prism' || l === 'javascript' || l === 'js' || l === 'text' || !l) {
    return highlightPrsm(code);
  }

  if (l === 'csharp' || l === 'cs' || l === 'c#') {
    return highlightCSharp(code);
  }

  return code;
};


const CodeBlock = ({ children, className }: { children: string; className?: string }) => {
  const [copied, setCopied] = useState(false);
  const language = className ? className.replace('language-', '') : 'prsm';

  const copyToClipboard = () => {
    navigator.clipboard.writeText(children.trim());
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const code = children.trim();
  const highlighted = highlightCode(code, language);
  const highlightedLines = highlighted.split('\n');

  return (
    <div className="brand-docs__code-wrapper">
      <div className="brand-docs__code-header">
        <span className="brand-docs__code-lang">{language.toUpperCase()}</span>
        <button 
          onClick={copyToClipboard} 
          className={`brand-docs__code-copy ${copied ? 'copied' : ''}`}
        >
          {copied ? 'Copied!' : 'Copy'}
        </button>
      </div>
      <div className="brand-docs__code-block">
        <pre className={className}>
          {code.split('\n').map((_, i) => (
            <div key={i} className="brand-docs__code-line">
              <span className="brand-docs__code-number">{i + 1}</span>
              <span 
                className="brand-docs__code-content"
                dangerouslySetInnerHTML={{ __html: highlightedLines[i] || ' ' }}
              />
            </div>
          ))}
        </pre>
      </div>
    </div>
  );
};


const renderInlineMarkdown = (text: string) => {
  // Split on: inline code, bold, links, and version markers
  return text.split(/(`[^`]+`|\*\*[^*]+\*\*|\[([^\]]+)\]\(([^)]+)\)|\(PrSM \d[\d.]* 부터\)|\([Ss]ince PrSM \d[\d.]*\))/).filter(Boolean).map((part, i) => {
    if (part.startsWith('`')) return <code key={i}>{part.slice(1, -1)}</code>;
    if (part.startsWith('**')) return <strong key={i}>{part.slice(2, -2)}</strong>;
    if (/^\((?:PrSM \d|[Ss]ince PrSM \d)/.test(part)) {
      return <span key={i} className="prsm-version-marker">{part}</span>;
    }
    // Links: [text](url) — captured groups become separate parts
    const linkMatch = part.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
    if (linkMatch) {
      return <a key={i} href={linkMatch[2]}>{linkMatch[1]}</a>;
    }
    return part;
  });
};

const renderTable = (rows: string[], key: string | number) => {
  const tableData = rows.map(row => 
    row.trim().split('|').filter(cell => cell.trim() !== '' || row.indexOf('|') !== row.lastIndexOf('|')).map(cell => cell.trim())
  );
  
  // Filter out the |---|---| separator row
  const header = tableData[0];
  const body = tableData.slice(2);

  return (
    <div className="brand-docs__table-wrapper" key={key}>
      <table>
        <thead>
          <tr>{header.map((cell, i) => <th key={i}>{renderInlineMarkdown(cell)}</th>)}</tr>
        </thead>
        <tbody>
          {body.map((row, i) => (
            <tr key={i}>{row.map((cell, j) => <td key={j}>{renderInlineMarkdown(cell)}</td>)}</tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

/**
 * A simple zero-dependency Markdown parser for basic doc features.
 */
function SimpleMarkdown({ content }: { content: string }) {
  const lines = content.split('\n');
  const elements: React.ReactNode[] = [];
  let inCodeBlock = false;
  let codeLines: string[] = [];
  let currentLang = '';
  
  let inList = false;
  let listItems: string[] = [];

  let inTable = false;
  let tableRows: string[] = [];

  const flushList = (key: number) => {
    if (listItems.length > 0) {
      elements.push(
        <ul key={`list-${key}`}>
          {listItems.map((item, idx) => <li key={idx}>{renderInlineMarkdown(item)}</li>)}
        </ul>
      );
      listItems = [];
      inList = false;
    }
  };

  const flushTable = (key: number) => {
    if (tableRows.length > 0) {
      elements.push(renderTable(tableRows, key));
      tableRows = [];
      inTable = false;
    }
  };

  lines.forEach((line, i) => {
    const trimmed = line.trim();

    // 1. Code Block handling
    if (line.startsWith('```')) {
      flushList(i);
      flushTable(i);
      if (inCodeBlock) {
        elements.push(
          <CodeBlock key={`code-${i}`} className={currentLang ? `language-${currentLang}` : ''}>
            {codeLines.join('\n')}
          </CodeBlock>
        );
        codeLines = [];
        inCodeBlock = false;
        currentLang = '';
      } else {
        inCodeBlock = true;
        currentLang = line.slice(3).trim();
      }
      return;
    }

    if (inCodeBlock) {
      codeLines.push(line);
      return;
    }

    // 2. Table handling
    const isTableRow = trimmed.startsWith('|') && trimmed.includes('|', trimmed.indexOf('|') + 1);
    if (isTableRow) {
      flushList(i);
      inTable = true;
      tableRows.push(line);
      return;
    } else if (inTable) {
      flushTable(i);
    }

    // 3. List handling (unordered: - or *, numbered: 1. 2. etc.)
    const numberedMatch = trimmed.match(/^(\d+)\.\s+(.+)/);
    if (trimmed.startsWith('- ') || trimmed.startsWith('* ') || numberedMatch) {
      flushTable(i);
      inList = true;
      listItems.push(numberedMatch ? numberedMatch[2] : trimmed.slice(2));
      return;
    } else if (inList) {
      flushList(i);
    }

    // 4. Blockquote — version cards
    if (trimmed.startsWith('> ')) {
      flushList(i);
      flushTable(i);
      const content = trimmed.slice(2);
      // Version marker blockquotes render as styled cards
      const isVersionCard = /(\[Since language \d|PrSM \d.*부터|[Ss]ince PrSM \d|Added in)/.test(content);
      if (isVersionCard) {
        elements.push(
          <div key={i} className="prsm-version-card">
            <span className="prsm-version-marker">{content.replace(/[\[\]]/g, '')}</span>
          </div>
        );
      } else {
        elements.push(
          <blockquote key={i}><p>{renderInlineMarkdown(content)}</p></blockquote>
        );
      }
      return;
    }

    // 5. Horizontal rules
    if (/^---+$/.test(trimmed) || /^\*\*\*+$/.test(trimmed)) {
      flushList(i);
      flushTable(i);
      elements.push(<hr key={i} />);
      return;
    }

    // 6. Headings & Paragraphs
    if (line.startsWith('#### ')) {
      elements.push(<h4 key={i}>{renderInlineMarkdown(line.slice(5))}</h4>);
    } else if (line.startsWith('# ')) {
      const text = line.slice(2);
      const id = text.toLowerCase().replace(/[^\w]+/g, '-');
      elements.push(<h1 key={i} id={id}>{renderInlineMarkdown(text)}</h1>);
    } else if (line.startsWith('## ')) {
      const text = line.slice(3);
      const id = text.toLowerCase().replace(/[^\w]+/g, '-');
      elements.push(<h2 key={i} id={id}>{renderInlineMarkdown(text)}</h2>);
    } else if (line.startsWith('### ')) {
      elements.push(<h3 key={i}>{renderInlineMarkdown(line.slice(4))}</h3>);
    } else if (line.trim()) {
      elements.push(<p key={i}>{renderInlineMarkdown(line)}</p>);
    }
  });

  flushList(lines.length);
  flushTable(lines.length);

  return <>{elements}</>;
}

export default function DocumentationView({ initialDocId = 'index', lang }: DocumentationViewProps) {
  const [activeDocId, setActiveDocId] = useState(initialDocId)
  const [content, setContent] = useState<string>('')
  const [loading, setLoading] = useState(true)
  const [isSidebarOpen, setIsSidebarOpen] = useState(false)
  const [activeSection, setActiveSection] = useState<string>('')

  const groups = useDocGroups(lang)
  const routes = groups.flatMap(g => g.items)
  const activeDoc = routes.find(r => r.id === activeDocId) || routes[0]

  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            setActiveSection(entry.target.id)
          }
        })
      },
      { rootMargin: '-80px 0% -60% 0%', threshold: 0 }
    )

    const headers = document.querySelectorAll('.brand-docs__article h1, .brand-docs__article h2')
    headers.forEach((header) => observer.observe(header))

    return () => observer.disconnect()
  }, [content])

  useEffect(() => {
    if (!activeDoc) return
    const fetchDoc = async () => {
      setLoading(true)
      try {
        const response = await fetch(import.meta.env.BASE_URL + activeDoc.path)
        if (!response.ok) throw new Error('Failed to load document')
        const text = await response.text()
        const cleanedText = text.replace(/^---[\s\S]+?---(\r?\n)+/, '')
        setContent(cleanedText)
      } catch (err) {
        setContent('# Error\nCould not load the documentation file.')
      } finally {
        setLoading(false)
      }
    }
    fetchDoc()
  }, [activeDocId, activeDoc])

  if (groups.length === 0) {
    return (
      <div className="brand-docs">
        <div className="brand-docs__loading">
          <div className="brand-docs__spinner" />
          <span>Loading navigation...</span>
        </div>
      </div>
    )
  }

  return (
    <div className="brand-docs">
      <div className="brand-docs__layout">
        <aside className={`brand-docs__sidebar ${isSidebarOpen ? 'brand-docs__sidebar--open' : ''}`}>
          <nav className="brand-docs__nav">
            {groups.map(group => (
              <div key={group.id} className="brand-docs__nav-group">
                <div className="brand-docs__nav-label">{group.title}</div>
                {group.items.map(route => (
                  <button
                    key={route.id}
                    onClick={() => {
                      setActiveDocId(route.id)
                      setIsSidebarOpen(false)
                    }}
                    className={`brand-docs__nav-item ${activeDocId === route.id ? 'brand-docs__nav-item--active' : ''}`}
                  >
                    <span>{route.title}</span>
                  </button>
                ))}
              </div>
            ))}
          </nav>
        </aside>

        {isSidebarOpen && (
          <div 
            className="brand-docs__overlay lg:hidden" 
            onClick={() => setIsSidebarOpen(false)} 
          />
        )}

        <main className="brand-docs__content">
          <div className="brand-docs__container">
            {loading ? (
              <div className="brand-docs__loading">
                <div className="brand-docs__spinner" />
                <span>Loading documentation...</span>
              </div>
            ) : (
              <article className="brand-docs__article" onClick={(e) => {
                // Intercept internal .md links and navigate within the docs viewer
                const target = e.target as HTMLElement;
                const anchor = target.closest('a');
                if (!anchor) return;
                const href = anchor.getAttribute('href');
                if (!href || !href.endsWith('.md')) return;
                e.preventDefault();
                // Extract filename without extension and path prefix
                const filename = href.replace(/^.*\//, '').replace(/\.md$/, '');
                // Find matching route by id or by path suffix
                const match = routes.find(r => r.id === filename || r.path.endsWith('/' + href) || r.path.endsWith(href));
                if (match) {
                  setActiveDocId(match.id);
                  window.scrollTo(0, 0);
                }
              }}>
                <SimpleMarkdown content={content} />
              </article>
            )}
          </div>
        </main>

        <aside className="brand-docs__toc">
          <div className="brand-docs__toc-items">
            {content.split('\n')
              .filter(line => line.startsWith('# ') || line.startsWith('## '))
              .map((line, i) => {
                const text = line.replace(/^#+ /, '')
                const id = text.toLowerCase().replace(/[^\w]+/g, '-')
                const level = line.startsWith('## ') ? 2 : 1
                return (
                  <a 
                    key={i} 
                    href={`#${id}`} 
                    className={`brand-docs__toc-item ${level === 2 ? 'pl-4' : ''} ${activeSection === id ? 'brand-docs__toc-item--active' : ''}`}
                    onClick={(e) => {
                      e.preventDefault()
                      setActiveSection(id)
                      document.getElementById(id)?.scrollIntoView({ behavior: 'smooth' })
                    }}
                  >
                    {text}
                  </a>
                )
              })}
          </div>
        </aside>
      </div>
    </div>
  )
}
