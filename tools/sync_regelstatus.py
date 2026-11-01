#!/usr/bin/env python3
"""Überträgt rust/tools/regelstatus.csv in die Fortschritts-Tabellen und die Doku.

- Forge_Rules_Engine.xlsx (Sheet „Rules“, Spalte G) und Forge_Rules/Forge_*.xlsx (Spalte F):
  ja → TRUE, n/a → Text "n/a", sonst FALSE.
- Summary-/Fortschrittsformeln zählen "n/a"-Zeilen nicht mehr mit.
- Erzeugt rust/docs/03-regelstand.md.

Nur Python-Standardbibliothek (kein openpyxl nötig). Aufruf aus dem Repo-Root:
    python3 rust/tools/sync_regelstatus.py
"""

import csv
import html
import re
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CSV = ROOT / "rust/tools/regelstatus.csv"
DOC = ROOT / "rust/docs/03-regelstand.md"
ENGINE = ROOT / "Forge_Rules_Engine.xlsx"
CATEGORY_FILES = {
    "Keyword": "Forge_Keyword.xlsx",
    "Trigger": "Forge_Trigger.xlsx",
    "Replacement Effect": "Forge_Replacement_Effect.xlsx",
    "Static Ability": "Forge_Static_Ability.xlsx",
    "Effect (ApiType)": "Forge_Effect_ApiType.xlsx",
    "Cost": "Forge_Cost.xlsx",
    "Turn Structure": "Forge_Turn_Structure.xlsx",
    "State-Based Action": "Forge_State_Based_Action.xlsx",
}
ORDER = list(CATEGORY_FILES)


def load_status():
    status = {}
    with CSV.open(encoding="utf-8") as f:
        rows = [line for line in f if line.strip() and not line.startswith("#")]
    for row in csv.reader(rows, delimiter=";"):
        row += [""] * (5 - len(row))
        cat, rule, st, tests, note = (x.strip() for x in row[:5])
        if st not in ("ja", "teilweise", "n/a"):
            sys.exit(f"Unbekannter Status '{st}' für {cat}/{rule}")
        status[(cat, rule)] = (st, tests, note)
    return status


ROW_RE = re.compile(r'(<row r="(\d+)"[^>]*>)(.*?)(</row>)', re.S)
CELL_RE = re.compile(r'<c r="([A-Z]+)(\d+)"([^>]*?)(?:/>|>(.*?)</c>)', re.S)


def cell_text(body):
    m = re.search(r"<t[^>]*>(.*?)</t>", body or "", re.S)
    if m:
        return html.unescape(m.group(1))
    m = re.search(r"<v>(.*?)</v>", body or "", re.S)
    return html.unescape(m.group(1)) if m else ""


def status_cell(ref, style, st):
    if st == "ja":
        return f'<c r="{ref}"{style} t="b"><v>1</v></c>'
    if st == "n/a":
        return f'<c r="{ref}"{style} t="inlineStr"><is><t>n/a</t></is></c>'
    return f'<c r="{ref}"{style} t="b"><v>0</v></c>'


def update_rules_sheet(xml, name_col, cat_col, status_col, fixed_cat, status, seen):
    rows_out = []

    def repl_row(m):
        head, rnum, body, tail = m.groups()
        if rnum == "1":
            return m.group(0)
        cells = {c.group(1): c for c in CELL_RE.finditer(body)}
        if name_col not in cells or status_col not in cells:
            return m.group(0)
        rule = cell_text(cells[name_col].group(4))
        cat = fixed_cat or cell_text(cells[cat_col].group(4))
        st = status.get((cat, rule), ("offen", "", ""))[0]
        if (cat, rule) in status:
            seen.add((cat, rule))
        rows_out.append((cat, rule))
        old = cells[status_col]
        style = re.search(r'\ss="\d+"', old.group(3))
        style = style.group(0) if style else ""
        new_cell = status_cell(f"{status_col}{rnum}", style, st)
        return head + body.replace(old.group(0), new_cell) + tail

    xml = ROW_RE.sub(repl_row, xml)
    xml = re.sub(r'<formula1>"TRUE,FALSE(?:,n/a)?"</formula1>', '<formula1>"TRUE,FALSE,n/a"</formula1>', xml)
    return xml, rows_out


def rewrite_zip(path, changes):
    tmp = path.with_suffix(".tmp.xlsx")
    with zipfile.ZipFile(path) as zin, zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
        for item in zin.infolist():
            data = changes.get(item.filename)
            zout.writestr(item, data.encode("utf-8") if data is not None else zin.read(item.filename))
    tmp.replace(path)


def descriptions():
    """(Kategorie, Regel) → (Beschreibung, Java-Quelle) aus dem Engine-Sheet."""
    with zipfile.ZipFile(ENGINE) as z:
        xml = z.read("xl/worksheets/sheet2.xml").decode("utf-8")
    out = {}
    for m in ROW_RE.finditer(xml):
        if m.group(2) == "1":
            continue
        cells = {c.group(1): cell_text(c.group(4)) for c in CELL_RE.finditer(m.group(3))}
        out[(cells.get("B", ""), cells.get("C", ""))] = (cells.get("D", ""), cells.get("F", ""))
    return out


def main():
    status = load_status()
    seen = set()

    # Gesamtdatei
    with zipfile.ZipFile(ENGINE) as z:
        rules = z.read("xl/worksheets/sheet2.xml").decode("utf-8")
        summary = z.read("xl/worksheets/sheet1.xml").decode("utf-8")
    rules, all_rows = update_rules_sheet(rules, "C", "B", "G", None, status, seen)
    summary = re.sub(
        r"<f>COUNTIF\(Rules!\$B\$2:\$B\$770,(A\d+)\)(?:-COUNTIFS\([^<]*\))?</f>",
        lambda m: f'<f>COUNTIF(Rules!$B$2:$B$770,{m.group(1)})-COUNTIFS(Rules!$B$2:$B$770,{m.group(1)},Rules!$G$2:$G$770,"n/a")</f>',
        summary,
    )
    summary = summary.replace(
        "How to use: set column G &apos;Translated to Rust&apos; on the Rules sheet to TRUE once a rule is ported.",
        "How to use: column G is maintained by rust/tools/sync_regelstatus.py (TRUE = ported + rule tests, n/a = out of scope, not counted).",
    ).replace(
        "How to use: set column G 'Translated to Rust' on the Rules sheet to TRUE once a rule is ported.",
        "How to use: column G is maintained by rust/tools/sync_regelstatus.py (TRUE = ported + rule tests, n/a = out of scope, not counted).",
    )
    rewrite_zip(ENGINE, {"xl/worksheets/sheet2.xml": rules, "xl/worksheets/sheet1.xml": summary})

    # Kategorie-Dateien
    for cat, fname in CATEGORY_FILES.items():
        path = ROOT / "Forge_Rules" / fname
        with zipfile.ZipFile(path) as z:
            xml = z.read("xl/worksheets/sheet1.xml").decode("utf-8")
        xml, _ = update_rules_sheet(xml, "B", None, "F", cat, status, seen)
        xml = re.sub(
            r"<f>COUNTA\((B2:B\d+)\)(?:-COUNTIF\([^<]*\))?</f>",
            lambda m: f'<f>COUNTA({m.group(1)})-COUNTIF({m.group(1).replace("B", "F")},"n/a")</f>',
            xml,
        )
        rewrite_zip(path, {"xl/worksheets/sheet1.xml": xml})

    unknown = set(status) - seen
    if unknown:
        sys.exit("Regeln aus regelstatus.csv nicht in den Tabellen gefunden: " + ", ".join(f"{c}/{r}" for c, r in sorted(unknown)))

    # Doku
    desc = descriptions()
    label = {"ja": "✅ ja", "teilweise": "🟡 teilweise", "n/a": "⚪ n/a", "offen": "offen"}
    lines = [
        "# Regelstand",
        "",
        "Automatisch erzeugt von `python3 rust/tools/sync_regelstatus.py` aus `rust/tools/regelstatus.csv`.",
        "Nicht von Hand bearbeiten – Status in der CSV pflegen.",
        "",
        "- **ja**: umgesetzt und durch Regeltests (Comprehensive Rules) abgesichert → in den xlsx-Dateien `TRUE`.",
        "- **teilweise**: Grundform umgesetzt, Varianten fehlen → `FALSE`.",
        "- **n/a**: gehört zu einem ausgeschlossenen Sonderformat → `n/a`, zählt nicht.",
        "- Der Abgleich mit der Java-Engine (Differential-Tests) steht noch aus, siehe `04-arbeitsprotokoll.md`.",
        "",
        "## Übersicht",
        "",
        "| Kategorie | Regeln (ohne n/a) | ja | teilweise | n/a |",
        "|---|---:|---:|---:|---:|",
    ]
    by_cat = {c: [r for (cc, r) in all_rows if cc == c] for c in ORDER}
    tot = [0, 0, 0, 0]
    for c in ORDER:
        sts = [status.get((c, r), ("offen",))[0] for r in by_cat[c]]
        n_na = sts.count("n/a")
        row = [len(sts) - n_na, sts.count("ja"), sts.count("teilweise"), n_na]
        tot = [a + b for a, b in zip(tot, row)]
        lines.append(f"| {c} | {row[0]} | {row[1]} | {row[2]} | {row[3]} |")
    lines.append(f"| **Summe** | **{tot[0]}** | **{tot[1]}** | **{tot[2]}** | **{tot[3]}** |")
    for c in ORDER:
        lines += ["", f"## {c}", "", "| Regel | Status | Test | Hinweis | Java-Quelle |", "|---|---|---|---|---|"]
        for r in by_cat[c]:
            st, tests, note = status.get((c, r), ("offen", "", ""))
            if st == "offen":
                continue
            java = desc.get((c, r), ("", ""))[1]
            esc = lambda s: s.replace("|", "\\|")
            test = f"`{tests}`" if tests else ""
            lines.append(f"| {esc(r)} | {label[st]} | {test} | {esc(note)} | {esc(java)} |")
        open_rules = [r for r in by_cat[c] if (c, r) not in status]
        if open_rules:
            lines += ["", f"Offen ({len(open_rules)}): " + ", ".join(open_rules)]
    DOC.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"aktualisiert: {ENGINE.name}, {len(CATEGORY_FILES)} Kategorie-Dateien, {DOC.relative_to(ROOT)}")
    print(f"ja={tot[1]} teilweise={tot[2]} n/a={tot[3]} von {tot[0]} (ohne n/a)")


if __name__ == "__main__":
    main()
