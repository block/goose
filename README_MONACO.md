# Monaco Editor Integration for Goose

## 📖 Overview

This branch (`spence/ideinput`) contains the planning and implementation for integrating **Monaco Editor** (VS Code's editor) into Goose's chat input, replacing the current `react-syntax-highlighter` implementation.

---

## 🎯 Goals

1. **Provide a professional IDE experience** for code input
2. **Enable advanced features** like autocomplete and IntelliSense
3. **Improve performance** for large code blocks
4. **Maintain backward compatibility** with existing features

---

## 📚 Documentation

### Planning Documents (Read in Order)

1. **[MONACO_INTEGRATION_SUMMARY.md](./MONACO_INTEGRATION_SUMMARY.md)** ⭐ START HERE
   - Executive summary
   - Current state vs proposed solution
   - Decision rationale
   - Timeline and metrics

2. **[MONACO_INTEGRATION_PLAN.md](./MONACO_INTEGRATION_PLAN.md)**
   - Detailed implementation roadmap
   - Phase-by-phase breakdown
   - Testing strategy
   - Migration path

3. **[MONACO_VS_SYNTAXHIGHLIGHTER.md](./MONACO_VS_SYNTAXHIGHLIGHTER.md)**
   - Feature comparison
   - Performance analysis
   - Use case evaluation
   - Decision matrix

4. **[MONACO_ARCHITECTURE.md](./MONACO_ARCHITECTURE.md)**
   - Component architecture
   - Data flow diagrams
   - State management
   - Integration points

5. **[MONACO_QUICK_START.md](./MONACO_QUICK_START.md)** 🚀 IMPLEMENTATION GUIDE
   - Step-by-step instructions
   - Code examples
   - Testing procedures
   - Troubleshooting

---

## 🏗️ Architecture

### High-Level Overview

```
ChatInput
  └── RichChatInput (dual-layer system)
      ├── Hidden Textarea (native input)
      └── Display Layer
          ├── Normal Mode
          │   ├── Text + Pills
          │   └── Spell Check
          └── Code Mode ⭐ NEW
              ├── Language Badge
              └── MonacoCodeInput
                  └── Monaco Editor
                      ├── Syntax Highlighting
                      ├── IntelliSense
                      ├── Autocomplete
                      └── Multi-cursor
```

### Key Components

| Component | Purpose | Status |
|-----------|---------|--------|
| `MonacoCodeInput.tsx` | Wrapper for Monaco Editor | 📋 To Create |
| `RichChatInput.tsx` | Main input component | ✏️ To Modify |
| `ChatInput.tsx` | Parent component | ✅ No Changes |

---

## 🚀 Quick Start

### For Reviewers

1. Read [MONACO_INTEGRATION_SUMMARY.md](./MONACO_INTEGRATION_SUMMARY.md)
2. Review [MONACO_VS_SYNTAXHIGHLIGHTER.md](./MONACO_VS_SYNTAXHIGHLIGHTER.md)
3. Check [MONACO_ARCHITECTURE.md](./MONACO_ARCHITECTURE.md)

### For Implementers

1. Read [MONACO_QUICK_START.md](./MONACO_QUICK_START.md)
2. Follow step-by-step instructions
3. Test thoroughly
4. Submit PR

### For Users

Once implemented:
1. Type `#python ` in chat input
2. See Monaco Editor appear
3. Enjoy IDE features!

---

## 📊 Key Metrics

### Current State (react-syntax-highlighter)
- ✅ Syntax highlighting: Static
- ❌ Autocomplete: None
- ❌ IntelliSense: None
- ⚠️ Performance: Slow for large code
- 📦 Bundle: ~50KB

### Target State (Monaco Editor)
- ✅ Syntax highlighting: Dynamic
- ✅ Autocomplete: Full support
- ✅ IntelliSense: Full support
- ✅ Performance: Fast for all sizes
- 📦 Bundle: ~3MB (lazy loaded from CDN)

### Success Criteria
- [ ] Monaco loads in <1 second
- [ ] Typing feels instant (60fps)
- [ ] Autocomplete works for all languages
- [ ] No regressions in existing features
- [ ] Positive user feedback

---

## 🎯 Implementation Status

### Phase 1: Planning ✅ COMPLETE
- [x] Create planning documents
- [x] Architecture design
- [x] Feature comparison
- [x] Risk analysis

### Phase 2: Setup ⏳ NEXT
- [ ] Install `@monaco-editor/react`
- [ ] Create `MonacoCodeInput.tsx`
- [ ] Basic Monaco rendering
- [ ] Test with simple code

### Phase 3: Integration 📋 PLANNED
- [ ] Integrate into `RichChatInput.tsx`
- [ ] Replace `SyntaxHighlighter`
- [ ] Update onChange handlers
- [ ] Add keyboard shortcuts

### Phase 4: Polish 📋 PLANNED
- [ ] Custom theme
- [ ] Height calculation
- [ ] Loading spinner
- [ ] Error boundaries

### Phase 5: Testing 📋 PLANNED
- [ ] Unit tests
- [ ] Integration tests
- [ ] E2E tests
- [ ] Performance profiling

### Phase 6: Deployment 📋 PLANNED
- [ ] Feature flag
- [ ] Beta testing
- [ ] Gradual rollout
- [ ] Monitor metrics

---

## 🔧 Technical Details

### Dependencies
```json
{
  "@monaco-editor/react": "^4.6.0",
  "monaco-editor": "peer dependency (auto-loaded)"
}
```

### Browser Support
- ✅ Chrome/Edge (latest)
- ✅ Firefox (latest)
- ✅ Safari (latest)
- ⚠️ Mobile (fallback to SyntaxHighlighter)

### Performance
- **Load Time**: ~500ms (first time)
- **Memory**: ~50MB
- **Bundle Size**: ~3MB (CDN cached)
- **Typing Latency**: <16ms (60fps)

---

## 🧪 Testing Strategy

### Unit Tests
- MonacoCodeInput component
- onChange handlers
- Keyboard shortcuts
- Height calculation

### Integration Tests
- Code mode activation
- Monaco loading
- Value synchronization
- Exit code mode

### E2E Tests
- Full user workflow
- All 30+ languages
- Performance benchmarks
- Accessibility

---

## 🚧 Known Issues & Limitations

### Current Limitations
1. **Bundle Size**: Monaco is ~3MB (mitigated by CDN)
2. **Load Time**: ~500ms initial load (mitigated by lazy loading)
3. **Mobile**: Not optimized (mitigated by fallback)

### Planned Improvements
1. Preload on hover over `#`
2. Custom autocomplete (Goose-specific)
3. Error highlighting (linting)
4. Format on paste

---

## 📈 Comparison with PR #5502

### What PR #5502 Has
- ✅ Rich text input
- ✅ Code mode with `#language`
- ✅ Syntax highlighting (static)
- ✅ Action pills
- ✅ Mention pills
- ✅ Spell checking

### What This Branch Adds
- ✅ Monaco Editor integration
- ✅ IntelliSense & autocomplete
- ✅ Multi-cursor editing
- ✅ Find/replace
- ✅ Better performance
- ✅ Professional IDE experience

### What Stays the Same
- ✅ Dual-layer architecture
- ✅ `#language` trigger
- ✅ Keyboard shortcuts (Enter, Cmd+Enter)
- ✅ Action pills
- ✅ Mention pills
- ✅ Spell checking

---

## 🤝 Contributing

### How to Help

1. **Review Planning Docs**
   - Provide feedback on architecture
   - Suggest improvements
   - Identify risks

2. **Test Implementation**
   - Try the code
   - Report bugs
   - Suggest UX improvements

3. **Write Tests**
   - Unit tests
   - Integration tests
   - E2E tests

4. **Improve Documentation**
   - Fix typos
   - Add examples
   - Clarify instructions

---

## 📞 Contact & Resources

### Team
- **Branch Owner**: Spencer Martin
- **Branch**: `spence/ideinput`
- **Base Branch**: `spence/textinput-experiment`

### Resources
- **Monaco Editor Docs**: https://microsoft.github.io/monaco-editor/
- **@monaco-editor/react**: https://github.com/suren-atoyan/monaco-react
- **Monaco Playground**: https://microsoft.github.io/monaco-editor/playground.html
- **PR #5502**: https://github.com/block/goose/pull/5502

---

## 🎉 Expected Outcome

### Before (Current)
```
User types: #python def hello(): print("hi")
Display: Static syntax highlighting, no autocomplete
```

### After (With Monaco)
```
User types: #python def hel
Display: Monaco Editor with:
  - Dynamic syntax highlighting
  - Autocomplete suggestions: hello(), help()
  - IntelliSense: function signature hints
  - Multi-cursor support
  - Find/replace
  - Professional IDE appearance
```

---

## 🎯 Next Steps

### Immediate
1. ✅ Review planning documents
2. ⏳ Get team alignment
3. ⏳ Begin implementation

### Short-term
1. ⏳ Create MonacoCodeInput component
2. ⏳ Integrate into RichChatInput
3. ⏳ Test and iterate

### Long-term
1. ⏳ Beta testing
2. ⏳ Gradual rollout
3. ⏳ Monitor and optimize

---

## 📊 Timeline

| Phase | Duration | Status |
|-------|----------|--------|
| Planning | 1 day | ✅ Complete |
| Setup | 1 day | ⏳ Next |
| Integration | 2 days | 📋 Planned |
| Polish | 1 day | 📋 Planned |
| Testing | 1 day | 📋 Planned |
| Deployment | Ongoing | 📋 Planned |

**Total Estimated Time**: 5-7 days

---

## ✅ Checklist

### Planning Phase ✅
- [x] Create MONACO_INTEGRATION_SUMMARY.md
- [x] Create MONACO_INTEGRATION_PLAN.md
- [x] Create MONACO_VS_SYNTAXHIGHLIGHTER.md
- [x] Create MONACO_ARCHITECTURE.md
- [x] Create MONACO_QUICK_START.md
- [x] Create README_MONACO.md

### Implementation Phase ⏳
- [ ] Install dependencies
- [ ] Create MonacoCodeInput component
- [ ] Integrate into RichChatInput
- [ ] Add styling
- [ ] Test functionality

### Testing Phase 📋
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Manual testing
- [ ] Performance profiling
- [ ] Accessibility testing

### Deployment Phase 📋
- [ ] Create feature flag
- [ ] Beta testing
- [ ] Gradual rollout
- [ ] Monitor metrics
- [ ] Full deployment

---

**Status**: 📋 Planning Complete - Ready for Implementation  
**Last Updated**: October 31, 2025  
**Version**: 1.0

---

## 🚀 Let's Build Something Amazing!

This integration will transform the Goose chat input into a professional IDE experience. Let's make it happen! 💪
