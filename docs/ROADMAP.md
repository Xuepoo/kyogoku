# Kyogoku Roadmap

Development roadmap for Kyogoku translation engine.

## Current Version: 0.1.0 ✓

**Release Date**: 2026-03-23

### Completed Features
- [x] CLI MVP with init/config/translate/cache commands
- [x] Multi-format parser (TXT, SRT, JSON)
- [x] Translation pipeline with caching
- [x] Glossary system
- [x] Multiple API provider support
- [x] Complete documentation suite

---

## Upcoming Milestones

### Q2 2026: Advanced Parsers

**Target**: Add support for game script formats using nom parser combinator library.

| Feature | Status | Details |
|---------|--------|---------|
| Ren'Py (.rpy) Parser | ✅ Completed | Visual novel script format, complex syntax |
| ASS/SSA Subtitles | ✅ Completed | Advanced SubStation Alpha format |
| WebVTT | ✅ Completed | Web Video Text Tracks standard |

**Implementation Plan:**
- Create `nom`-based lexer for syntax analysis
- Design AST (Abstract Syntax Tree) for Ren'Py
- Preserve script control flow (jumps, conditionals)
- Add format-specific test suites

**Expected Effort**: 4-6 weeks

---

### Q3 2026: GUI Application

**Target**: Create desktop GUI using Tauri 2.0 with real-time progress visualization.

| Feature | Status | Details |
|---------|--------|---------|
| Tauri 2.0 App | 🔄 Planned | Desktop application framework |
| File Browser | 🔄 Planned | Interactive file/folder selection |
| Progress Dashboard | 🔄 Planned | Real-time translation stats |
| Translation History | 🔄 Planned | View and manage translations |
| Settings Panel | 🔄 Planned | GUI config editor |

**Tech Stack:**
- Frontend: Vanilla JavaScript + DOM API (lightweight)
- Backend: Existing Rust crates via Tauri IPC
- Build: Tauri 2.0 with vite (optional)

**Expected Effort**: 6-8 weeks

---

### Q4 2026: RAG (Retrieval-Augmented Generation)

**Target**: Add intelligent context retrieval using local vector database.

| Feature | Status | Details |
|---------|--------|---------|
| Embedding Generation | 🔄 Planned | Local embeddings (Ollama/ONNX) |
| Vector Database | 🔄 Planned | Qdrant or Milvus integration |
| Character Profiles | 🔄 Planned | Auto-retrieval of character info |
| Context Injection | 🔄 Planned | Dynamic context based on content |

**Workflow:**
1. Analyze translated content for entities (characters, locations)
2. Store embeddings in vector DB
3. On new file: retrieve similar past translations
4. Inject as system context for consistency

**Expected Effort**: 6-8 weeks

---

## Future Considerations (2027+)

- [ ] Cloud synchronization for team projects
- [ ] Model fine-tuning pipeline for specific genres
- [ ] Plugin system for custom transformations
- [ ] Web-based interface
- [ ] Collaborative translation workflows
- [ ] Translation quality metrics and analytics

---

## Dependency Timeline

```
v0.1.0 ✓ (Done)
  │
  ├─→ v0.2.0 (Q2 2026: .rpy/.ass parsers)
  │     │
  │     └─→ v0.3.0 (Q3 2026: Tauri GUI)
  │           │
  │           └─→ v0.4.0 (Q4 2026: RAG)
  │                 │
  │                 └─→ v1.0.0 (Stable release)
  │
  └─→ Feature branches (parallel development)
```

---

## How to Contribute

See [DEVELOPER.md](DEVELOPER.md) for:
- Setting up development environment
- Running tests
- Submitting pull requests

### Areas Needing Help
- [ ] Test coverage improvement
- [ ] Performance optimization
- [ ] Documentation refinement
- [ ] Platform-specific testing (Windows, macOS)
- [ ] Translation quality evaluation

---

## Known Limitations

| Limitation | Impact | Workaround |
|-----------|--------|-----------|
| No batch API retry | Large files may fail mid-translation | Use `--no-cache` to retry |
| Limited to sequential translation | Slow for large files | Increase `max_concurrency` |
| No format validation | Corrupted output possible | Manual review recommended |
| Context window naive | Inconsistent long documents | Increase `context_size` |

---

*Last updated: 2026-03-23*
