import { useState, useEffect } from 'react'
import { BookOpen } from 'lucide-react'
import Prism from './components/Prism.jsx'
import prsmLogo from './assets/prsm-logo.svg'
import TrueFocus from './components/TrueFocus'
import DocumentationView from './components/DocumentationView'
import './App.css'

const BRAND_CONTENT = {
  en: {
    home: 'Home',
    docs: 'Docs',
    heroTitle: 'A prism of ideas for',
    heroFocus: 'fast Unity scripting',
    heroDesc: 'PrSM bends concise syntax into generated C#, Unity tooling, and source-aware editor workflows.',
    getStarted: 'Get Started',
    documentation: 'Documentation',
  },
  ko: {
    home: '홈',
    docs: '문서',
    heroTitle: '아이디어의 프리즘,',
    heroFocus: '빠른 Unity 스크립팅',
    heroDesc: 'PrSM은 간결한 문법을 C# 코드로 변환하며, Unity 툴링 및 소스 맵 기반의 에디터 워크플로우를 제공합니다.',
    getStarted: '시작하기',
    documentation: '문서 보기',
  }
}

function App() {
  const [view, setView] = useState<'landing' | 'docs'>('landing')
  const [activeDoc, setActiveDoc] = useState('index')
  const [lang, setLang] = useState<'en' | 'ko'>('en')

  const t = BRAND_CONTENT[lang]

  // Handle back/forward navigation or initial load
  useEffect(() => {
    const path = window.location.pathname
    if (path.includes('/en/')) {
      setLang('en')
      setView('docs')
      const docId = path.split('/en/')[1].replace('.html', '') || 'index'
      setActiveDoc(docId)
    } else if (path.includes('/ko/')) {
      setLang('ko')
      setView('docs')
      const docId = path.split('/ko/')[1].replace('.html', '') || 'index'
      setActiveDoc(docId)
    }
  }, [])

  const enterDocs = (docId = 'index', newLang = lang) => {
    setLang(newLang)
    setActiveDoc(docId)
    setView('docs')
    // Construct absolute path from root to prevent stacking (e.g. /en/en/)
    const path = `/${newLang}/${docId === 'index' ? '' : docId + '.html'}`.replace(/\/+/g, '/')
    window.history.pushState({}, '', path)
  }

  return (
    <main className={`brand-page ${view === 'docs' ? 'brand-page--docs' : ''}`}>
      {view === 'landing' && (
        <>
          <div className="brand-prism-stage" aria-hidden="true">
            <Prism
              height={3.5}
              baseWidth={5.5}
              animationType="rotate"
              glow={1}
              noise={0}
              transparent
              scale={3}
              hueShift={0}
              colorFrequency={1}
              hoverStrength={2}
              inertia={0.05}
              bloom={1}
              timeScale={0.5}
            />
          </div>
          <div className="brand-vignette" aria-hidden="true" />
          <div className="brand-floor-glow" aria-hidden="true" />
        </>
      )}

      <header className="brand-nav">
        <div className="brand-nav__left">
          <a className="brand-mark" href="./" onClick={(e) => { e.preventDefault(); setView('landing'); window.history.pushState({}, '', '/') }}>
            <img src={prsmLogo} alt="PrSM logo" className="brand-mark__logo" />
            <span>PrSM</span>
          </a>

          <nav className="brand-nav__links" aria-label="Primary">
            <a 
              className={`brand-nav__link ${view === 'landing' ? 'brand-nav__link--active' : ''}`} 
              href="./"
              onClick={(e) => { e.preventDefault(); setView('landing'); window.history.pushState({}, '', '/') }}
            >
              {t.home}
            </a>
            <a 
              className={`brand-nav__link ${view === 'docs' ? 'brand-nav__link--active' : ''}`} 
              href={`./${lang}/`}
              onClick={(e) => { e.preventDefault(); enterDocs('index', lang) }}
            >
              {t.docs}
            </a>
          </nav>
        </div>

        <div className="brand-nav__right">
          <div className="brand-nav__langs">
            <button 
              className={`brand-nav__lang ${lang === 'ko' ? 'brand-nav__lang--active' : ''}`}
              onClick={() => setLang('ko')}
            >
              KO
            </button>
            <span className="brand-nav__lang-sep">/</span>
            <button 
              className={`brand-nav__lang ${lang === 'en' ? 'brand-nav__lang--active' : ''}`}
              onClick={() => setLang('en')}
            >
              EN
            </button>
          </div>
        </div>
      </header>

      {view === 'docs' ? (
        <DocumentationView 
          initialDocId={activeDoc} 
          lang={lang} 
        />
      ) : (
        <section className="brand-hero">
        <div className="brand-badge">
          <img src={prsmLogo} alt="" className="brand-badge__logo" />
          <span className="brand-badge__text">
            <span className="brand-badge__text--bold">PrSM</span>
            <span>Script</span>
          </span>
        </div>

        <div className="brand-copy">
          <h1 className="brand-title">
            <span>{t.heroTitle}</span>
            <TrueFocus 
              sentence={t.heroFocus} 
              borderColor="#786eff" 
              glowColor="rgba(120, 110, 255, 0.4)"
              blurAmount={2}
              animationDuration={0.8}
            />
          </h1>
          <p>{t.heroDesc}</p>
        </div>

        <div className="brand-actions">
          <a 
            className="brand-button brand-button--primary" 
            href={`./${lang}/getting-started.html`}
            onClick={(e) => { e.preventDefault(); enterDocs('getting-started', lang) }}
          >
            <BookOpen size={18} strokeWidth={2} />
            <span>{t.getStarted}</span>
          </a>
        </div>
      </section>
    )}
  </main>
)
}

export default App
