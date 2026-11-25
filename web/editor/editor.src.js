// CodeMirror setup. It lives in a shadow root, where CodeMirror uses constructed
// style sheets, so the page keeps `style-src 'self'`.
import { EditorState, Compartment } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { basicSetup } from "codemirror";
import { indentWithTab } from "@codemirror/commands";
import { indentUnit } from "@codemirror/language";
import { autocompletion, completeAnyWord } from "@codemirror/autocomplete";
import { lintGutter, setDiagnostics } from "@codemirror/lint";
import { cpp } from "@codemirror/lang-cpp";
import { java } from "@codemirror/lang-java";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { oneDark } from "@codemirror/theme-one-dark";

const LANG = { c: cpp, cpp, java, js: javascript, python, rust };

/** Completes `obs`, `obs[`, `obs.` and `s->` into a whole field access. */
function fieldSource(list) {
  return (ctx) => {
    const m = ctx.matchBefore(/\b(?:obs(?:\.|\["?|\.get\w*\(?"?|\.int\(?"?|\.grid\(?"?)?|s(?:->?)?)\w*$/);
    if (!m) return null;
    return { from: m.from, filter: false, options: list.filter((c) => c.label.startsWith(m.text) || c.detail.startsWith(/\w*$/.exec(m.text)[0])).map((c) => ({
      ...c, type: "property",
      apply(view, _c, from, to) {
        // Drop the closing brackets that the editor added on its own.
        const after = view.state.sliceDoc(to, to + 3), auto = /^["\])]+/.exec(after)?.[0] ?? "";
        const opened = /[["(]/.test(view.state.sliceDoc(from, to));
        view.dispatch({ changes: { from, to: opened ? to + auto.length : to, insert: c.label }, selection: { anchor: from + c.label.length } });
      },
    })) };
  };
}
const words = (ctx) => {
  const r = completeAnyWord(ctx);
  if (!r || ctx.matchBefore(/(?:obs(?:\.|\[)|s->)\w*$/)) return null;
  return { ...r, options: r.options.filter((o) => !/^\d/.test(o.label)) };
};

export function createEditor(host, { onChange, onRun, label }) {
  const root = host.attachShadow({ mode: "open" });
  const mode = new Compartment(), theme = new Compartment(), lock = new Compartment();
  let quiet = false;
  const base = EditorView.theme({
    "&": { height: "100%", fontSize: "13px" },
    ".cm-scroller": { fontFamily: 'ui-monospace, Consolas, Menlo, monospace', lineHeight: "1.5" },
  });
  const make = (doc) => EditorState.create({
    doc,
    extensions: [
      basicSetup, indentUnit.of("    "), EditorState.tabSize.of(4), lintGutter(), base,
      keymap.of([{ key: "Mod-Enter", run: () => { onRun(); return true; } }, indentWithTab]),
      EditorView.contentAttributes.of({ "aria-label": label }),
      EditorView.updateListener.of((u) => { if (u.docChanged && !quiet) onChange(u.state.doc.toString()); }),
      mode.of([]), theme.of([]), lock.of([]),
    ],
  });
  const view = new EditorView({ state: make(""), root, parent: root });
  return {
    /** Load a document. Completions are [{label, detail, info, apply?}]. */
    open(code, lang, completions, dark) {
      quiet = true;
      view.setState(make(code));
      view.dispatch({ effects: [
        mode.reconfigure([LANG[lang]?.() ?? [], autocompletion({ override: [fieldSource(completions), words] })]),
        theme.reconfigure(dark ? oneDark : []),
      ] });
      quiet = false;
    },
    text: () => view.state.doc.toString(),
    setText(t) { view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: t } }); },
    insert(t) { view.dispatch(view.state.replaceSelection(t)); view.focus(); },
    lock(v) { view.dispatch({ effects: lock.reconfigure(EditorState.readOnly.of(v)) }); },
    /** errors: [{line, message}] with 1-based lines. */
    mark(errors) {
      const doc = view.state.doc;
      view.dispatch(setDiagnostics(view.state, errors.filter((e) => e.line >= 1 && e.line <= doc.lines).map((e) => {
        const l = doc.line(e.line);
        // renderMessage is not used: CodeMirror shows `message` as plain text.
        return { from: l.from, to: l.to, severity: "error", message: e.message };
      })));
      const first = errors.find((e) => e.line >= 1 && e.line <= doc.lines);
      if (first) view.dispatch({ effects: EditorView.scrollIntoView(doc.line(first.line).from, { y: "center" }) });
    },
    marks: () => root.querySelectorAll(".cm-lint-marker-error").length,
  };
}
