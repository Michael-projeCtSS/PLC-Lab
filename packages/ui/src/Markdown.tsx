/**
 * Safe Markdown subset renderer for knowledge content. Produces React elements only:
 * no HTML injection, no navigation. Supported: headings, paragraphs, bullet and
 * numbered lists, fenced code, tables, inline code, bold, italic and links (shown as
 * text with the address, because the desktop view never navigates away).
 */
import { Fragment, type ReactNode } from "react";

function inline(text: string, key: string): ReactNode[] {
  const out: ReactNode[] = [];
  const pattern =
    /(`[^`]+`)|(\*\*[^*]+\*\*)|(_[^_]+_)|(\[[^\]]+\]\([^)\s]+\))/g;
  let last = 0;
  let match: RegExpExecArray | null;
  let n = 0;
  while ((match = pattern.exec(text)) !== null) {
    if (match.index > last) out.push(text.slice(last, match.index));
    const token = match[0];
    const k = `${key}-${n++}`;
    if (token.startsWith("`"))
      out.push(<code key={k}>{token.slice(1, -1)}</code>);
    else if (token.startsWith("**"))
      out.push(<strong key={k}>{token.slice(2, -2)}</strong>);
    else if (token.startsWith("_"))
      out.push(<em key={k}>{token.slice(1, -1)}</em>);
    else {
      const label = token.slice(1, token.indexOf("]"));
      const url = token.slice(token.indexOf("(") + 1, -1);
      out.push(
        <span key={k} title={url}>
          {label} <span className="dim">({url})</span>
        </span>,
      );
    }
    last = match.index + token.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

function cells(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((c) => c.trim());
}

export function Markdown({ text }: { text: string }) {
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  const blocks: ReactNode[] = [];
  let i = 0;
  let key = 0;
  while (i < lines.length) {
    const line = lines[i] ?? "";
    const k = `b${key++}`;
    if (!line.trim()) {
      i++;
    } else if (line.startsWith("```")) {
      const code: string[] = [];
      i++;
      while (i < lines.length && !(lines[i] ?? "").startsWith("```"))
        code.push(lines[i++] ?? "");
      i++;
      blocks.push(
        <pre key={k}>
          <code>{code.join("\n")}</code>
        </pre>,
      );
    } else if (/^#{1,4} /.test(line)) {
      const level = line.indexOf(" ");
      const content = inline(line.slice(level + 1), k);
      blocks.push(
        level <= 2 ? <h3 key={k}>{content}</h3> : <h4 key={k}>{content}</h4>,
      );
      i++;
    } else if (/^\s*[-*] /.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\s*[-*] /.test(lines[i] ?? ""))
        items.push((lines[i++] ?? "").replace(/^\s*[-*] /, ""));
      blocks.push(
        <ul key={k}>
          {items.map((it, j) => (
            <li key={j}>{inline(it, `${k}-${j}`)}</li>
          ))}
        </ul>,
      );
    } else if (/^\s*\d+\. /.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\s*\d+\. /.test(lines[i] ?? ""))
        items.push((lines[i++] ?? "").replace(/^\s*\d+\. /, ""));
      blocks.push(
        <ol key={k}>
          {items.map((it, j) => (
            <li key={j}>{inline(it, `${k}-${j}`)}</li>
          ))}
        </ol>,
      );
    } else if (line.trim().startsWith("|")) {
      const rows: string[][] = [];
      while (i < lines.length && (lines[i] ?? "").trim().startsWith("|")) {
        const row = lines[i++] ?? "";
        if (!/^\s*\|?\s*:?-{2,}/.test(row)) rows.push(cells(row));
      }
      const [head, ...body] = rows;
      blocks.push(
        <table key={k}>
          {head && (
            <thead>
              <tr>
                {head.map((c, j) => (
                  <th key={j}>{inline(c, `${k}-h${j}`)}</th>
                ))}
              </tr>
            </thead>
          )}
          <tbody>
            {body.map((r, ri) => (
              <tr key={ri}>
                {r.map((c, j) => (
                  <td key={j}>{inline(c, `${k}-${ri}-${j}`)}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>,
      );
    } else {
      const para: string[] = [];
      while (
        i < lines.length &&
        (lines[i] ?? "").trim() &&
        !/^(#{1,4} |```|\s*[-*] |\s*\d+\. |\s*\|)/.test(lines[i] ?? "")
      )
        para.push(lines[i++] ?? "");
      blocks.push(<p key={k}>{inline(para.join(" "), k)}</p>);
    }
  }
  return <Fragment>{blocks}</Fragment>;
}
