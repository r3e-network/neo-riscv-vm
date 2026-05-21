import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(fileURLToPath(import.meta.url));
const W = 1800;
const H = 1100;

const c = {
  bg: "#f7f9fc",
  panel: "#ffffff",
  ink: "#101828",
  text: "#344054",
  muted: "#667085",
  line: "#d0d5dd",
  grid: "#e6edf5",
  green: "#16a34a",
  greenSoft: "#eafaf0",
  blue: "#2f80ed",
  blueSoft: "#edf5ff",
  purple: "#7c3aed",
  purpleSoft: "#f3ecff",
  amber: "#f59e0b",
  amberSoft: "#fff7df",
  teal: "#0097a7",
  tealSoft: "#e7fbfd",
  red: "#e5484d",
  redSoft: "#fff0f1",
  slate: "#475467",
  slateSoft: "#f2f4f7",
};

const sets = {
  en: {
    set: "English Set",
    generated: "Generated SVG diagrams. Edit generate-diagrams.mjs, then rerun node docs/diagrams/generate-diagrams.mjs.",
    intro: "Professional bilingual diagrams for explaining how NeoVM compatibility runs inside Neo's RISC-V execution stack.",
    title1: "System Architecture",
    sub1: "Neo stays the semantic authority; the adapter and Rust host provide a sandboxed RISC-V execution stack.",
    title2: "Execution Routing",
    sub2: "One Neo invocation surface routes legacy NeoVM bytecode and native RISC-V binaries to the right execution path.",
    title3: "NeoVM Inside RISC-V",
    sub3: "Existing NeoVM bytecode runs unchanged through the shared neo-vm-rs interpreter behind the PolkaVM boundary.",
    title4: "Syscall and Native Contract Data Flow",
    sub4: "RISC-V code asks for host operations; existing Neo C# executes storage, runtime, contract calls, and native logic.",
    title5: "ABI and Memory Model",
    sub5: "The FFI envelope becomes guest-readable aux data; the result is marshaled back as a Neo-compatible stack.",
    title6: "Contract Deployment Workflow",
    sub6: "Build, package, deploy, detect contract type, persist metadata, and route future invocations automatically.",
    title7: "Mainnet State Root Validation",
    sub7: "Continuous validation watches node height, checkpoints, state roots, logs, and automatic recovery actions.",
  },
  zh: {
    set: "中文图集",
    generated: "生成式 SVG 图集。修改 generate-diagrams.mjs 后运行 node docs/diagrams/generate-diagrams.mjs 重新生成。",
    intro: "用于解释 NeoVM 兼容层如何运行在 Neo RISC-V 执行栈中的专业中英文图解。",
    title1: "系统总体架构",
    sub1: "Neo 继续负责链语义；Adapter 和 Rust Host 提供沙盒化 RISC-V 执行栈。",
    title2: "执行路由",
    sub2: "同一个 Neo 调用入口把旧 NeoVM 字节码和原生 RISC-V 二进制路由到正确执行路径。",
    title3: "NeoVM 运行在 RISC-V 中",
    sub3: "现有 NeoVM 字节码无需修改，因为 PolkaVM 边界后复用共享的 neo-vm-rs 解释器。",
    title4: "Syscall 与原生合约数据流",
    sub4: "RISC-V 代码请求 host 操作；现有 Neo C# 执行存储、运行时、合约调用和原生逻辑。",
    title5: "ABI 与内存模型",
    sub5: "FFI 调用信封变成 guest 可读 aux data；执行结果再封送为 Neo 兼容结果栈。",
    title6: "合约部署工作流",
    sub6: "构建、打包、部署、识别合约类型、持久化元数据，并自动路由后续调用。",
    title7: "主网 StateRoot 校验",
    sub7: "持续校验节点高度、checkpoint、state root、日志和自动恢复动作。",
  },
};

function esc(s) {
  return String(s)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function cjk(s) {
  return /[\u3400-\u9fff]/.test(s);
}

function visualLen(s) {
  return Array.from(String(s)).reduce((n, ch) => n + (/[\u3400-\u9fff]/.test(ch) ? 1.15 : 0.72), 0);
}

function wrap(s, max = 34) {
  const text = String(s ?? "").trim();
  if (!text) return [];
  if (text.length <= max) return [text];
  if (cjk(text)) {
    const tokens = text.match(/[A-Za-z0-9_.:+/#()\\-]+|[\u3400-\u9fff]|[^\u3400-\u9fffA-Za-z0-9_.:+/#()\\-\s]|\s+/g) ?? [];
    const lines = [];
    let line = "";
    for (const token of tokens) {
      const normalized = /^\s+$/.test(token) ? " " : token;
      const candidate = `${line}${normalized}`;
      if (visualLen(candidate) > max && line.trim()) {
        lines.push(line.trim());
        line = normalized.trimStart();
      } else {
        line = candidate;
      }
      if (/[，。；：、]/.test(normalized) && visualLen(line) >= max * 0.78) {
        lines.push(line.trim());
        line = "";
      }
    }
    if (line.trim()) lines.push(line.trim());
    return lines;
  }
  const words = text.split(/\s+/);
  const lines = [];
  let line = "";
  for (const word of words) {
    const next = line ? `${line} ${word}` : word;
    if (next.length > max && line) {
      lines.push(line);
      line = word;
    } else {
      line = next;
    }
  }
  if (line) lines.push(line);
  return lines;
}

function text(s, x, y, opts = {}) {
  const {
    size = 22,
    weight = 400,
    fill = c.text,
    max = 36,
    lh = Math.round(size * 1.32),
    anchor = "start",
    family = "Inter, Noto Sans CJK SC, Microsoft YaHei, PingFang SC, Arial, sans-serif",
  } = opts;
  return wrap(s, max)
    .map((line, i) => `<text x="${x}" y="${y + i * lh}" text-anchor="${anchor}" font-family="${family}" font-size="${size}" font-weight="${weight}" fill="${fill}">${esc(line)}</text>`)
    .join("\n");
}

function textBlock(items, x, y, opts = {}) {
  const {
    size = 18,
    weight = 400,
    fill = c.text,
    max = 34,
    lh = Math.round(size * 1.34),
    anchor = "start",
    family = "Inter, Noto Sans CJK SC, Microsoft YaHei, PingFang SC, Arial, sans-serif",
    gap = 4,
  } = opts;
  const raw = Array.isArray(items) ? items : [items];
  const lines = [];
  for (const item of raw) {
    const wrapped = wrap(item, max);
    if (lines.length && wrapped.length) lines.push({ text: "", gap: true });
    for (const line of wrapped) lines.push({ text: line, gap: false });
  }

  let offset = 0;
  return lines
    .map((line) => {
      if (line.gap) {
        offset += gap;
        return "";
      }
      const out = `<text x="${x}" y="${y + offset}" text-anchor="${anchor}" font-family="${family}" font-size="${size}" font-weight="${weight}" fill="${fill}">${esc(line.text)}</text>`;
      offset += lh;
      return out;
    })
    .join("\n");
}

function grid() {
  return `<defs>
    <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
      <path d="M 40 0 L 0 0 0 40" fill="none" stroke="${c.grid}" stroke-width="1"/>
    </pattern>
    <marker id="arrow-slate" markerWidth="14" markerHeight="14" refX="10" refY="7" orient="auto" markerUnits="strokeWidth"><path d="M2,2 L12,7 L2,12 z" fill="${c.slate}"/></marker>
    <marker id="arrow-green" markerWidth="14" markerHeight="14" refX="10" refY="7" orient="auto" markerUnits="strokeWidth"><path d="M2,2 L12,7 L2,12 z" fill="${c.green}"/></marker>
    <marker id="arrow-blue" markerWidth="14" markerHeight="14" refX="10" refY="7" orient="auto" markerUnits="strokeWidth"><path d="M2,2 L12,7 L2,12 z" fill="${c.blue}"/></marker>
    <marker id="arrow-red" markerWidth="14" markerHeight="14" refX="10" refY="7" orient="auto" markerUnits="strokeWidth"><path d="M2,2 L12,7 L2,12 z" fill="${c.red}"/></marker>
    <marker id="arrow-purple" markerWidth="14" markerHeight="14" refX="10" refY="7" orient="auto" markerUnits="strokeWidth"><path d="M2,2 L12,7 L2,12 z" fill="${c.purple}"/></marker>
    <filter id="soft-shadow" x="-10%" y="-10%" width="120%" height="130%">
      <feDropShadow dx="0" dy="10" stdDeviation="12" flood-color="#101828" flood-opacity="0.10"/>
    </filter>
  </defs>`;
}

function icon(name, x, y, size, color) {
  const s = size;
  const stroke = `stroke="${color}" stroke-width="${Math.max(2, s / 18)}" stroke-linecap="round" stroke-linejoin="round"`;
  if (name === "chain") {
    return `<g fill="none" ${stroke}>
      <rect x="${x + s * .10}" y="${y + s * .32}" width="${s * .38}" height="${s * .28}" rx="${s * .14}"/>
      <rect x="${x + s * .52}" y="${y + s * .32}" width="${s * .38}" height="${s * .28}" rx="${s * .14}"/>
      <path d="M ${x + s * .42} ${y + s * .46} L ${x + s * .58} ${y + s * .46}"/>
    </g>`;
  }
  if (name === "plugin") {
    return `<g fill="none" ${stroke}>
      <path d="M ${x + s * .22} ${y + s * .38} h ${s * .20} v ${-s * .10} a ${s * .09} ${s * .09} 0 0 1 ${s * .18} 0 v ${s * .10} h ${s * .20} v ${s * .20} h ${s * .10} a ${s * .09} ${s * .09} 0 0 1 0 ${s * .18} h ${-s * .10} v ${s * .20} h ${-s * .20} v ${s * .10} a ${s * .09} ${s * .09} 0 0 1 ${-s * .18} 0 v ${-s * .10} h ${-s * .20} z"/>
    </g>`;
  }
  if (name === "gear") {
    return `<g fill="none" ${stroke}>
      <circle cx="${x + s / 2}" cy="${y + s / 2}" r="${s * .20}"/>
      <path d="M ${x + s / 2} ${y + s * .08} v ${s * .16} M ${x + s / 2} ${y + s * .76} v ${s * .16} M ${x + s * .08} ${y + s / 2} h ${s * .16} M ${x + s * .76} ${y + s / 2} h ${s * .16} M ${x + s * .20} ${y + s * .20} l ${s * .12} ${s * .12} M ${x + s * .68} ${y + s * .68} l ${s * .12} ${s * .12} M ${x + s * .80} ${y + s * .20} l ${-s * .12} ${s * .12} M ${x + s * .32} ${y + s * .68} l ${-s * .12} ${s * .12}"/>
    </g>`;
  }
  if (name === "shield") {
    return `<g fill="none" ${stroke}>
      <path d="M ${x + s * .50} ${y + s * .10} L ${x + s * .84} ${y + s * .24} v ${s * .28} c 0 ${s * .24} ${-s * .14} ${s * .36} ${-s * .34} ${s * .46} c ${-s * .20} ${-s * .10} ${-s * .34} ${-s * .22} ${-s * .34} ${-s * .46} v ${-s * .28} z"/>
      <path d="M ${x + s * .36} ${y + s * .52} l ${s * .10} ${s * .10} l ${s * .22} ${-s * .26}"/>
    </g>`;
  }
  if (name === "vm") {
    return `<g fill="none" ${stroke}>
      <rect x="${x + s * .12}" y="${y + s * .16}" width="${s * .76}" height="${s * .62}" rx="${s * .08}"/>
      <path d="M ${x + s * .12} ${y + s * .32} h ${s * .76} M ${x + s * .30} ${y + s * .50} l ${-s * .08} ${s * .08} l ${s * .08} ${s * .08} M ${x + s * .48} ${y + s * .66} l ${s * .08} ${-s * .32} M ${x + s * .64} ${y + s * .50} l ${s * .08} ${s * .08} l ${-s * .08} ${s * .08}"/>
    </g>`;
  }
  if (name === "storage") {
    return `<g fill="none" ${stroke}>
      <ellipse cx="${x + s / 2}" cy="${y + s * .24}" rx="${s * .34}" ry="${s * .13}"/>
      <path d="M ${x + s * .16} ${y + s * .24} v ${s * .45} c 0 ${s * .08} ${s * .15} ${s * .13} ${s * .34} ${s * .13} c ${s * .19} 0 ${s * .34} ${-s * .05} ${s * .34} ${-s * .13} v ${-s * .45}"/>
      <path d="M ${x + s * .16} ${y + s * .46} c 0 ${s * .08} ${s * .15} ${s * .13} ${s * .34} ${s * .13} c ${s * .19} 0 ${s * .34} ${-s * .05} ${s * .34} ${-s * .13}"/>
    </g>`;
  }
  if (name === "memory") {
    return `<g fill="none" ${stroke}>
      <rect x="${x + s * .16}" y="${y + s * .14}" width="${s * .68}" height="${s * .72}" rx="${s * .06}"/>
      ${[.25, .39, .53, .67].map((yy) => `<path d="M ${x + s * .16} ${y + s * yy} h ${s * .68}"/>`).join("")}
      <path d="M ${x + s * .30} ${y + s * .04} v ${s * .10} M ${x + s * .50} ${y + s * .04} v ${s * .10} M ${x + s * .70} ${y + s * .04} v ${s * .10} M ${x + s * .30} ${y + s * .86} v ${s * .10} M ${x + s * .50} ${y + s * .86} v ${s * .10} M ${x + s * .70} ${y + s * .86} v ${s * .10}"/>
    </g>`;
  }
  if (name === "monitor") {
    return `<g fill="none" ${stroke}>
      <rect x="${x + s * .12}" y="${y + s * .18}" width="${s * .76}" height="${s * .52}" rx="${s * .06}"/>
      <path d="M ${x + s * .30} ${y + s * .82} h ${s * .40} M ${x + s * .50} ${y + s * .70} v ${s * .12} M ${x + s * .24} ${y + s * .56} l ${s * .16} ${-s * .16} l ${s * .14} ${s * .10} l ${s * .20} ${-s * .22}"/>
    </g>`;
  }
  return `<circle cx="${x + s / 2}" cy="${y + s / 2}" r="${s * .35}" fill="none" ${stroke}/>`;
}

function svg(title, subtitle, accent, body) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(title)}">
  ${grid()}
  <rect width="${W}" height="${H}" fill="${c.bg}"/>
  <rect width="${W}" height="${H}" fill="url(#grid)" opacity="0.42"/>
  <rect x="54" y="46" width="1692" height="1008" rx="28" fill="${c.panel}" stroke="${c.line}" stroke-width="1.5" filter="url(#soft-shadow)"/>
  <rect x="54" y="46" width="1692" height="10" rx="5" fill="${accent}"/>
  <circle cx="106" cy="98" r="22" fill="${accent}" opacity="0.16"/>
  <circle cx="106" cy="98" r="8" fill="${accent}"/>
  ${text(title, 142, 96, { size: 38, weight: 850, fill: c.ink, max: 52 })}
  ${text(subtitle, 142, 132, { size: 19, fill: c.muted, max: 105 })}
  ${body}
</svg>
`;
}

function box({ x, y, w, h, title, body = [], iconName = "vm", accent = c.blue, fill = c.blueSoft, max = 30, compact = false }) {
  const iconSize = compact ? 42 : 54;
  const titleX = x + 28 + iconSize + 18;
  const titleY = y + (compact ? 42 : 52);
  const titleSize = compact ? 21 : 24;
  const titleLh = Math.round(titleSize * 1.32);
  const titleLines = wrap(title, max).length;
  const bodyY = titleY + (titleLines - 1) * titleLh + (compact ? 31 : 36);
  const lines = Array.isArray(body) ? body : [body];
  const bodySize = compact ? 15 : 17;
  const bodyMax = Math.max(18, max + (compact ? 7 : 9));
  return `<g filter="url(#soft-shadow)">
    <rect x="${x}" y="${y}" width="${w}" height="${h}" rx="18" fill="${fill}" stroke="${accent}" stroke-width="2"/>
    <rect x="${x + 22}" y="${y + 20}" width="${iconSize}" height="${iconSize}" rx="14" fill="#ffffff" stroke="${accent}" stroke-width="1.5"/>
    ${icon(iconName, x + 22 + iconSize * .16, y + 20 + iconSize * .16, iconSize * .68, accent)}
    ${text(title, titleX, titleY, { size: titleSize, weight: 800, fill: c.ink, max, lh: titleLh })}
    ${textBlock(lines, x + 30, bodyY, { size: bodySize, fill: c.text, max: bodyMax, lh: compact ? 21 : 23, gap: compact ? 5 : 6 })}
  </g>`;
}

function miniBox(x, y, w, h, label, accent, fill) {
  return `<g>
    <rect x="${x}" y="${y}" width="${w}" height="${h}" rx="14" fill="${fill}" stroke="${accent}" stroke-width="1.5"/>
    ${text(label, x + w / 2, y + h / 2 + 7, { size: 18, weight: 800, fill: c.ink, max: Math.floor(w / 10), anchor: "middle" })}
  </g>`;
}

function step(x, y, n, label, detail, accent = c.blue, fill = c.blueSoft) {
  const titleSize = 22;
  const titleLh = 27;
  const titleMax = 16;
  const bodyY = y + 88 + Math.max(0, wrap(label, titleMax).length - 1) * 18;
  return `<g filter="url(#soft-shadow)">
    <rect x="${x}" y="${y}" width="250" height="146" rx="18" fill="${fill}" stroke="${accent}" stroke-width="2"/>
    <circle cx="${x + 38}" cy="${y + 38}" r="20" fill="${accent}"/>
    ${text(n, x + 38, y + 46, { size: 20, weight: 850, fill: "#fff", anchor: "middle", max: 2 })}
    ${text(label, x + 70, y + 43, { size: titleSize, weight: 850, fill: c.ink, max: titleMax, lh: titleLh })}
    ${textBlock(detail, x + 28, bodyY, { size: 14, fill: c.text, max: 24, lh: 19 })}
  </g>`;
}

function callout(x, y, w, h, title, detail, accent = c.green, fill = c.greenSoft) {
  const titleMax = Math.floor(w / 13);
  const titleLh = Math.round(22 * 1.32);
  const bodyY = y + 58 + (wrap(title, titleMax).length - 1) * titleLh + 34;
  return `<g>
    <rect x="${x}" y="${y}" width="${w}" height="${h}" rx="16" fill="${fill}" stroke="${accent}" stroke-width="1.8"/>
    <path d="M ${x + 22} ${y + 24} h 42" stroke="${accent}" stroke-width="5" stroke-linecap="round"/>
    ${text(title, x + 24, y + 58, { size: 22, weight: 850, fill: c.ink, max: titleMax, lh: titleLh })}
    ${textBlock(detail, x + 24, bodyY, { size: 15, fill: c.text, max: Math.floor(w / 13), lh: 21 })}
  </g>`;
}

function diamond(x, y, w, h, title, detail, accent = c.amber) {
  const p = `${x + w / 2},${y} ${x + w},${y + h / 2} ${x + w / 2},${y + h} ${x},${y + h / 2}`;
  return `<g filter="url(#soft-shadow)">
    <polygon points="${p}" fill="${c.amberSoft}" stroke="${accent}" stroke-width="2"/>
    ${text(title, x + w / 2, y + h / 2 - 6, { size: 23, weight: 850, fill: c.ink, max: 18, anchor: "middle" })}
    ${text(detail, x + w / 2, y + h / 2 + 28, { size: 15, fill: c.text, max: 24, anchor: "middle" })}
  </g>`;
}

function arrow(x1, y1, x2, y2, label = "", color = c.slate, marker = "slate", curve = 0) {
  const d = curve ? `M ${x1} ${y1} C ${x1 + curve} ${y1}, ${x2 - curve} ${y2}, ${x2} ${y2}` : `M ${x1} ${y1} L ${x2} ${y2}`;
  const mx = (x1 + x2) / 2;
  const my = (y1 + y2) / 2 - 12;
  const labelW = label ? Math.min(260, Math.max(120, visualLen(label) * 8.8 + 42)) : 0;
  const labelMax = label ? Math.max(14, Math.floor(labelW / 8.5)) : 0;
  const labelLines = label ? wrap(label, labelMax).length : 0;
  const labelH = label ? 22 + labelLines * 17 : 0;
  const labelY = my - labelH + 10;
  const labelSvg = label
    ? `<g><rect x="${mx - labelW / 2}" y="${labelY}" width="${labelW}" height="${labelH}" rx="8" fill="${c.panel}" stroke="${c.line}" opacity="0.96"/>${text(label, mx, labelY + 23, { size: 14, weight: 800, fill: color, anchor: "middle", max: labelMax, lh: 17 })}</g>`
    : "";
  return `<g>
    <path d="${d}" fill="none" stroke="${color}" stroke-width="3.2" stroke-linecap="round" marker-end="url(#arrow-${marker})"/>
    ${labelSvg}
  </g>`;
}

function lane(x, y, w, h, title, accent) {
  const labelW = Math.min(w - 60, Math.max(220, visualLen(title) * 10.5 + 48));
  const labelH = wrap(title, Math.floor(labelW / 12)).length > 1 ? 54 : 38;
  return `<g>
    <rect x="${x}" y="${y}" width="${w}" height="${h}" rx="22" fill="#fff" stroke="${accent}" stroke-width="2" stroke-dasharray="10 9"/>
    <rect x="${x + 22}" y="${y - 24}" width="${labelW}" height="${labelH}" rx="12" fill="${c.panel}" stroke="${accent}" stroke-width="1.5"/>
    ${text(title, x + 44, y + 1, { size: 16, weight: 850, fill: accent, max: Math.floor(labelW / 12), lh: 20 })}
  </g>`;
}

function render1(lang) {
  const zh = lang === "zh";
  const t = sets[lang];
  const body = `
    ${box({ x: 110, y: 175, w: 420, h: 155, title: zh ? "用户、dApp、合约" : "Users, dApps, Contracts", body: [zh ? "RPC / 交易 / 旧 NeoVM NEF / 原生 RISC-V NEF" : "RPC, transactions, legacy NeoVM NEF, native RISC-V NEF"], iconName: "chain", accent: c.green, fill: c.greenSoft, max: 28 })}
    ${box({ x: 690, y: 170, w: 490, h: 165, title: zh ? "Neo Core / Node（C#）" : "Neo Core / Node (C#)", body: [zh ? "ApplicationEngine、账本、存储、gas、权限、原生合约" : "ApplicationEngine, ledger, storage, gas, permissions, native contracts"], iconName: "storage", accent: c.blue, fill: c.blueSoft, max: 27 })}
    ${box({ x: 1295, y: 175, w: 360, h: 155, title: zh ? "Adapter 插件" : "Adapter Plugin", body: [zh ? "注册 Provider；P/Invoke 调 Rust host" : "Registers provider; calls Rust host via P/Invoke"], iconName: "plugin", accent: c.purple, fill: c.purpleSoft, max: 19 })}
    ${arrow(530, 252, 690, 252, zh ? "Neo 调用" : "Neo invoke", c.green, "green")}
    ${arrow(1180, 252, 1295, 252, "Provider", c.purple, "purple")}
    ${box({ x: 310, y: 475, w: 500, h: 190, title: zh ? "Rust Host Runtime" : "Rust Host Runtime", body: [zh ? "FFI、缓存、PolkaVM Engine、实例池、ABI 封送" : "FFI, cache, PolkaVM engine, instance pool, ABI marshaling"], iconName: "gear", accent: c.amber, fill: c.amberSoft, max: 34 })}
    ${box({ x: 990, y: 475, w: 500, h: 190, title: zh ? "PolkaVM / RISC-V Guest" : "PolkaVM / RISC-V Guest", body: [zh ? "guest.polkavm 转接 neo-vm-rs；PVM 可直跑" : "guest.polkavm bridges to neo-vm-rs; PVM runs directly"], iconName: "shield", accent: c.teal, fill: c.tealSoft, max: 34 })}
    ${arrow(1475, 330, 670, 475, "P/Invoke", c.slate, "slate", 360)}
    ${arrow(810, 563, 990, 563, zh ? "沙盒执行" : "sandbox execute", c.teal, "blue")}
    ${callout(150, 760, 430, 170, zh ? "C# 是语义事实源" : "C# owns Neo semantics", zh ? "syscall、原生合约、权限、gas 与链状态不在 Rust 里重写。" : "Syscalls, native contracts, permissions, gas, and chain state remain in C#.", c.blue, c.blueSoft)}
    ${callout(685, 760, 430, 170, zh ? "Rust 负责执行边界" : "Rust owns execution boundaries", zh ? "缓存、FFI、PolkaVM 沙盒、结果封送和诊断放在 host runtime。" : "Caching, FFI, PolkaVM sandboxing, result marshaling, and diagnostics live in the host runtime.", c.amber, c.amberSoft)}
    ${callout(1220, 760, 430, 170, zh ? "协议保持 Neo N3 兼容" : "Neo N3 compatibility is preserved", zh ? "交易格式、RPC、SDK、旧合约调用方式对用户保持一致。" : "Transaction format, RPC, SDKs, and legacy contract invocation stay consistent for users.", c.green, c.greenSoft)}
  `;
  return svg(t.title1, t.sub1, c.blue, body);
}

function render2(lang) {
  const zh = lang === "zh";
  const t = sets[lang];
  const body = `
    ${box({ x: 100, y: 175, w: 330, h: 135, title: zh ? "Neo 调用入口" : "Neo Invocation", body: [zh ? "ApplicationEngine.Run()" : "ApplicationEngine.Run()"], iconName: "chain", accent: c.green, fill: c.greenSoft, max: 18, compact: true })}
    ${arrow(430, 242, 560, 242, "", c.slate, "slate")}
    ${box({ x: 560, y: 175, w: 360, h: 135, title: zh ? "RiscvApplicationEngine" : "RiscvApplicationEngine", body: [zh ? "收集上下文、初始栈、gas" : "Collects context, initial stack, gas"], iconName: "plugin", accent: c.blue, fill: c.blueSoft, max: 23, compact: true })}
    ${arrow(920, 242, 1060, 242, "", c.slate, "slate")}
    ${diamond(1060, 160, 260, 165, zh ? "合约类型？" : "Contract Type?", zh ? "Type / PVM\\0 magic" : "Type / PVM\\0 magic", c.amber)}
    ${lane(155, 420, 675, 285, zh ? "旧 NeoVM 兼容路径" : "Legacy NeoVM Path", c.teal)}
    ${lane(970, 420, 675, 285, zh ? "原生 RISC-V 路径" : "Native RISC-V Path", c.purple)}
    ${arrow(1120, 325, 495, 420, zh ? "NeoVM = 0" : "NeoVM = 0", c.teal, "blue", -240)}
    ${arrow(1260, 325, 1305, 420, zh ? "RiscV = 1" : "RiscV = 1", c.purple, "purple")}
    ${box({ x: 210, y: 475, w: 285, h: 135, title: "guest.polkavm", body: [zh ? "加载缓存 guest facade" : "Load cached guest facade"], iconName: "vm", accent: c.teal, fill: c.tealSoft, compact: true })}
    ${arrow(495, 542, 585, 542, "", c.teal, "blue")}
    ${box({ x: 585, y: 475, w: 210, h: 135, title: zh ? "解释 NEF" : "Interpret NEF", body: [zh ? "NeoVM bytecode" : "NeoVM bytecode"], iconName: "vm", accent: c.teal, fill: c.tealSoft, compact: true, max: 12 })}
    ${box({ x: 1030, y: 475, w: 285, h: 135, title: zh ? "加载 PVM" : "Load PVM", body: [zh ? "NEF 内 PolkaVM binary" : "PolkaVM binary inside NEF"], iconName: "shield", accent: c.purple, fill: c.purpleSoft, compact: true })}
    ${arrow(1315, 542, 1405, 542, "", c.purple, "purple")}
    ${box({ x: 1405, y: 475, w: 205, h: 135, title: zh ? "直接执行" : "Run Directly", body: [zh ? "无 NeoVM 兼容层" : "No NeoVM compatibility layer"], iconName: "gear", accent: c.purple, fill: c.purpleSoft, compact: true, max: 13 })}
    ${arrow(690, 610, 870, 805, "", c.slate, "slate", 120)}
    ${arrow(1320, 610, 995, 805, "", c.slate, "slate", -160)}
    ${box({ x: 690, y: 805, w: 420, h: 130, title: zh ? "共享 Host Bridge" : "Shared Host Bridge", body: [zh ? "host_call + host_on_instruction + ABI" : "host_call + host_on_instruction + ABI"], iconName: "gear", accent: c.amber, fill: c.amberSoft, compact: true, max: 23 })}
    ${arrow(1110, 870, 1280, 870, zh ? "结果" : "result", c.green, "green")}
    ${box({ x: 1280, y: 805, w: 330, h: 130, title: zh ? "返回 C#" : "Return to C#", body: [zh ? "HALT/FAULT、栈、gas、异常" : "HALT/FAULT, stack, gas, exception"], iconName: "chain", accent: c.green, fill: c.greenSoft, compact: true, max: 18 })}
  `;
  return svg(t.title2, t.sub2, c.green, body);
}

function render3(lang) {
  const zh = lang === "zh";
  const t = sets[lang];
  const body = `
    ${box({ x: 105, y: 185, w: 310, h: 170, title: zh ? "NeoVM 字节码" : "NeoVM Bytecode", body: ["PUSH / CALL / SYSCALL / RET", zh ? "现有 NEF 无需改" : "Existing NEF stays unchanged"], iconName: "vm", accent: c.green, fill: c.greenSoft, max: 18 })}
    ${arrow(415, 270, 560, 270, zh ? "脚本输入" : "script input", c.green, "green")}
    <g filter="url(#soft-shadow)">
      <rect x="560" y="160" width="900" height="640" rx="28" fill="${c.tealSoft}" stroke="${c.teal}" stroke-width="3"/>
      <rect x="590" y="205" width="840" height="88" rx="20" fill="#fff" stroke="${c.teal}" stroke-width="1.5"/>
      ${icon("shield", 620, 222, 50, c.teal)}
      ${text(zh ? "PolkaVM 沙盒边界" : "PolkaVM Sandbox Boundary", 690, 242, { size: 27, weight: 850, fill: c.ink, max: 34 })}
      ${text(zh ? "guest memory 隔离，只有 host imports 可离开沙盒" : "guest memory is isolated; host imports are the only exit", 690, 273, { size: 17, fill: c.text, max: 56 })}
      <rect x="640" y="350" width="710" height="315" rx="24" fill="#fff" stroke="${c.line}" stroke-width="1.5"/>
      ${text("guest.polkavm", 675, 400, { size: 28, weight: 850, fill: c.ink, max: 22 })}
      ${miniBox(690, 450, 160, 70, zh ? "取指" : "Fetch", c.blue, c.blueSoft)}
      ${arrow(850, 485, 930, 485, "", c.slate, "slate")}
      ${miniBox(930, 450, 170, 70, zh ? "解码" : "Decode", c.purple, c.purpleSoft)}
      ${arrow(1100, 485, 1180, 485, "", c.slate, "slate")}
      ${miniBox(1180, 450, 150, 70, zh ? "执行" : "Execute", c.green, c.greenSoft)}
      ${arrow(1260, 520, 770, 598, zh ? "下一 opcode" : "next opcode", c.slate, "slate", -260)}
      ${miniBox(690, 585, 220, 58, zh ? "Evaluation Stack" : "Evaluation Stack", c.amber, c.amberSoft)}
      ${miniBox(940, 585, 170, 58, zh ? "Locals" : "Locals", c.amber, c.amberSoft)}
      ${miniBox(1140, 585, 190, 58, zh ? "Call/Try Frames" : "Call/Try Frames", c.amber, c.amberSoft)}
    </g>
    ${box({ x: 1510, y: 270, w: 210, h: 165, title: zh ? "Host Imports" : "Host Imports", body: ["host_call()", "host_on_instruction()"], iconName: "gear", accent: c.red, fill: c.redSoft, max: 12, compact: true })}
    ${arrow(1350, 505, 1510, 352, zh ? "syscall / gas" : "syscall / gas", c.red, "red", 80)}
    ${box({ x: 1190, y: 845, w: 500, h: 160, title: zh ? "回到 Neo C# 系统" : "Back to Neo C# System", body: [zh ? "storage、runtime、native call 仍由 C# 执行" : "storage, runtime, and native calls still execute in C#"], iconName: "chain", accent: c.blue, fill: c.blueSoft, compact: true, max: 27 })}
    ${arrow(1615, 435, 1460, 865, zh ? "请求 Neo 语义" : "request Neo semantics", c.blue, "blue", -120)}
    ${callout(105, 780, 420, 170, zh ? "用户友好理解" : "User mental model", zh ? "不是把旧合约改写成 RISC-V；而是通过 PolkaVM 边界复用 neo-vm-rs 语义。" : "Legacy contracts are not rewritten to RISC-V; they reuse neo-vm-rs semantics through the PolkaVM boundary.", c.green, c.greenSoft)}
  `;
  return svg(t.title3, t.sub3, c.teal, body);
}

function render4(lang) {
  const zh = lang === "zh";
  const t = sets[lang];
  const cols = [
    [120, zh ? "RISC-V Guest" : "RISC-V Guest", c.teal],
    [485, zh ? "Rust Host" : "Rust Host", c.amber],
    [850, zh ? "C# Bridge" : "C# Bridge", c.purple],
    [1215, zh ? "Neo C# System" : "Neo C# System", c.blue],
  ];
  const headers = cols.map(([x, label, color]) => `${miniBox(x, 185, 260, 58, label, color, "#fff")}<path d="M ${x + 130} 243 V 820" stroke="${color}" stroke-width="2" stroke-dasharray="8 10" opacity="0.55"/>`).join("\n");
  const body = `
    ${headers}
    ${arrow(250, 320, 615, 320, "1. host_call(api, ip, stack)", c.teal, "blue")}
    ${arrow(615, 430, 980, 430, zh ? "2. P/Invoke callback" : "2. P/Invoke callback", c.amber, "slate")}
    ${arrow(980, 540, 1345, 540, zh ? "3. 分发 syscall / native" : "3. dispatch syscall / native", c.purple, "purple")}
    ${arrow(1345, 650, 980, 650, zh ? "4. StackItem 结果" : "4. StackItem result", c.green, "green")}
    ${arrow(980, 760, 615, 760, zh ? "5. ABI bytes" : "5. ABI bytes", c.green, "green")}
    ${arrow(615, 870, 250, 870, zh ? "6. guest 继续执行" : "6. guest continues", c.green, "green")}
    ${box({ x: 1130, y: 300, w: 430, h: 155, title: zh ? "Neo 语义事实源" : "Neo Semantic Authority", body: ["Storage / Runtime / Contract.Call / Crypto", zh ? "Ledger、NEO、GAS、Policy、Oracle 等原生合约" : "Ledger, NEO, GAS, Policy, Oracle, and other native contracts"], iconName: "storage", accent: c.blue, fill: c.blueSoft, max: 27, compact: true })}
    ${callout(170, 890, 1340, 145, zh ? "架构原则" : "Architecture Rule", zh ? "RISC-V 侧只负责转发和数据适配，不维护第二套 syscall 或原生合约实现。" : "The RISC-V side only forwards and adapts data; it does not maintain a second syscall or native-contract implementation.", c.red, c.redSoft)}
  `;
  return svg(t.title4, t.sub4, c.purple, body);
}

function memorySegment(x, y, w, h, label, detail, color, fill) {
  return `<g>
    <rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${fill}" stroke="${color}" stroke-width="1.4"/>
    ${text(label, x + 20, y + 32, { size: 19, weight: 850, fill: c.ink, max: 22 })}
    ${text(detail, x + 20, y + 58, { size: 15, fill: c.text, max: 31, lh: 20 })}
  </g>`;
}

function render5(lang) {
  const zh = lang === "zh";
  const t = sets[lang];
  const body = `
    ${box({ x: 105, y: 170, w: 380, h: 280, title: zh ? "FFI 调用信封" : "FFI Call Envelope", body: ["script_ptr / len", "initial_ip / trigger / network", "gas_left / timestamp", "initial_stack / callbacks"], iconName: "plugin", accent: c.blue, fill: c.blueSoft, max: 23 })}
    ${arrow(485, 310, 650, 310, zh ? "序列化" : "serialize", c.blue, "blue")}
    <g filter="url(#soft-shadow)">
      <rect x="650" y="160" width="500" height="760" rx="24" fill="#fff" stroke="${c.teal}" stroke-width="2.4"/>
      ${icon("memory", 690, 196, 60, c.teal)}
      ${text(zh ? "Guest Memory Map" : "Guest Memory Map", 770, 236, { size: 28, weight: 850, fill: c.ink, max: 30 })}
      ${memorySegment(725, 290, 350, 80, zh ? "Reserved / Entry" : "Reserved / Entry", zh ? "入口点与 trampoline" : "entry points and trampolines", c.slate, c.slateSoft)}
      ${memorySegment(725, 370, 350, 100, zh ? "Code" : "Code", zh ? "guest.polkavm 或 PVM binary，只读" : "guest.polkavm or PVM binary, read-only", c.blue, c.blueSoft)}
      ${memorySegment(725, 470, 350, 170, zh ? "Heap" : "Heap", zh ? "256MB bump allocator；StackValue / Array / Map / Struct" : "256MB bump allocator; StackValue / Array / Map / Struct", c.green, c.greenSoft)}
      ${memorySegment(725, 640, 350, 110, zh ? "Aux Data" : "Aux Data", zh ? "script、初始栈与上下文元数据" : "script, initial stack, context metadata", c.amber, c.amberSoft)}
      ${memorySegment(725, 750, 350, 105, zh ? "Execution Stack" : "Execution Stack", zh ? "向下增长；栈与 item size 有限制" : "grows down; stack and item size are bounded", c.purple, c.purpleSoft)}
    </g>
    ${arrow(1150, 310, 1320, 310, zh ? "编码/解码" : "encode/decode", c.amber, "slate")}
    ${box({ x: 1320, y: 170, w: 370, h: 280, title: zh ? "StackValue ABI" : "StackValue ABI", body: ["Integer / BigInteger", "ByteString / Boolean / Null", "Array / Struct / Map", "Interop / Iterator / Pointer"], iconName: "vm", accent: c.amber, fill: c.amberSoft, max: 23 })}
    ${box({ x: 105, y: 625, w: 380, h: 250, title: zh ? "执行结果" : "Execution Result", body: [zh ? "state：HALT / FAULT / BREAK" : "state: HALT / FAULT / BREAK", zh ? "结果栈、gas、异常信息" : "result stack, gas, exception data", zh ? "由 FFI free callback 释放" : "released by FFI free callback"], iconName: "chain", accent: c.green, fill: c.greenSoft, max: 22 })}
    ${arrow(650, 760, 485, 750, zh ? "封送回 C#" : "marshal back", c.green, "green")}
    ${callout(1320, 625, 370, 190, zh ? "安全边界" : "Safety Boundary", zh ? "guest 不能直接访问链状态；所有外部行为必须通过 host_call 和 C# dispatcher。" : "The guest cannot directly access chain state; all external behavior must go through host_call and the C# dispatcher.", c.red, c.redSoft)}
  `;
  return svg(t.title5, t.sub5, c.amber, body);
}

function render6(lang) {
  const zh = lang === "zh";
  const t = sets[lang];
  const body = `
    ${step(95, 185, "1", zh ? "构建" : "Build", zh ? "Rust RISC-V 或旧 NeoVM 产物" : "Rust RISC-V or legacy NeoVM artifact", c.green, c.greenSoft)}
    ${arrow(345, 258, 445, 258, "", c.slate, "slate")}
    ${step(445, 185, "2", zh ? "打包" : "Package", zh ? "PVM / NeoVM payload + manifest => NEF" : "PVM or NeoVM payload + manifest => NEF", c.teal, c.tealSoft)}
    ${arrow(695, 258, 795, 258, "", c.slate, "slate")}
    ${step(795, 185, "3", zh ? "部署" : "Deploy", zh ? "Deploy 写入链上合约状态" : "Deploy writes contract state", c.blue, c.blueSoft)}
    ${arrow(1045, 258, 1145, 258, "", c.slate, "slate")}
    ${step(1145, 185, "4", zh ? "识别类型" : "Detect Type", zh ? "PVM\\0 magic 设置 ContractState.Type" : "PVM\\0 magic sets ContractState.Type", c.purple, c.purpleSoft)}
    ${arrow(1395, 258, 1495, 258, "", c.slate, "slate")}
    ${step(1495, 185, "5", zh ? "存储" : "Persist", zh ? "hash、NEF、manifest、Type 入链" : "hash, NEF, manifest, Type stored", c.amber, c.amberSoft)}
    ${diamond(760, 515, 280, 165, zh ? "调用时路由" : "Route on Invoke", zh ? "NeoVM=0 / RiscV=1" : "NeoVM=0 / RiscV=1", c.amber)}
    ${arrow(1630, 331, 900, 515, zh ? "读取 Type" : "read Type", c.amber, "slate", 300)}
    ${box({ x: 210, y: 705, w: 430, h: 150, title: zh ? "NeoVM 兼容执行" : "NeoVM Compatibility Execution", body: [zh ? "加载 guest.polkavm，转接 neo-vm-rs 执行 NEF" : "Load guest.polkavm and route NEF execution to neo-vm-rs"], iconName: "vm", accent: c.teal, fill: c.tealSoft, compact: true, max: 25 })}
    ${box({ x: 1160, y: 705, w: 430, h: 150, title: zh ? "原生 RISC-V 执行" : "Native RISC-V Execution", body: [zh ? "直接加载 PVM binary，不经过 NeoVM 兼容层" : "Load PVM binary directly without the NeoVM compatibility layer"], iconName: "shield", accent: c.purple, fill: c.purpleSoft, compact: true, max: 25 })}
    ${arrow(760, 600, 640, 740, zh ? "NeoVM" : "NeoVM", c.teal, "blue", -120)}
    ${arrow(1040, 600, 1160, 740, zh ? "RISC-V" : "RISC-V", c.purple, "purple", 120)}
    ${box({ x: 685, y: 865, w: 430, h: 120, title: zh ? "共享 C# 语义层" : "Shared C# Semantics", body: [zh ? "syscalls / native contracts / storage / gas" : "syscalls / native contracts / storage / gas"], iconName: "chain", accent: c.blue, fill: c.blueSoft, compact: true, max: 23 })}
    ${arrow(640, 800, 685, 915, "", c.blue, "blue", 60)}
    ${arrow(1160, 800, 1115, 915, "", c.blue, "blue", -60)}
  `;
  return svg(t.title6, t.sub6, c.green, body);
}

function render7(lang) {
  const zh = lang === "zh";
  const t = sets[lang];
  const body = `
    ${box({ x: 110, y: 170, w: 390, h: 155, title: zh ? "Node 同步" : "Node Sync", body: [zh ? "neo-cli 主网节点同步 blocks / state；RPC 输出 blockcount" : "neo-cli mainnet node syncs blocks / state; RPC exposes blockcount"], iconName: "chain", accent: c.green, fill: c.greenSoft, max: 24, compact: true })}
    ${arrow(500, 248, 705, 248, zh ? "高度" : "height", c.green, "green")}
    ${box({ x: 705, y: 170, w: 390, h: 155, title: zh ? "StateRoot Comparator" : "StateRoot Comparator", body: [zh ? "按区间比较 expected 与 actual state root" : "Compares expected and actual state roots by range"], iconName: "gear", accent: c.blue, fill: c.blueSoft, max: 24, compact: true })}
    ${arrow(1095, 248, 1300, 248, zh ? "状态" : "status", c.blue, "blue")}
    ${box({ x: 1300, y: 170, w: 390, h: 155, title: "Watchdog", body: [zh ? "轮询服务、日志、checkpoint，必要时触发恢复" : "Polls services, logs, checkpoint, and triggers recovery when needed"], iconName: "monitor", accent: c.purple, fill: c.purpleSoft, max: 24, compact: true })}
    ${box({ x: 250, y: 465, w: 380, h: 150, title: "stateroot-continuous.tsv", body: [zh ? "height / expected / actual / OK 或 MISMATCH" : "height / expected / actual / OK or MISMATCH"], iconName: "storage", accent: c.teal, fill: c.tealSoft, max: 24, compact: true })}
    ${box({ x: 710, y: 465, w: 380, h: 150, title: "checkpoint", body: [zh ? "记录最后通过区块，重启后继续前进" : "records last passed block and resumes after restart"], iconName: "memory", accent: c.amber, fill: c.amberSoft, max: 24, compact: true })}
    ${box({ x: 1170, y: 465, w: 380, h: 150, title: "stdout / stderr", body: [zh ? "过滤 Unknown state root、fatal、mismatch 等异常" : "filters Unknown state root, fatal errors, mismatch, and anomalies"], iconName: "vm", accent: c.red, fill: c.redSoft, max: 24, compact: true })}
    ${arrow(900, 325, 440, 465, zh ? "写 TSV" : "write TSV", c.teal, "blue", -220)}
    ${arrow(900, 325, 900, 465, zh ? "更新" : "update", c.amber, "slate")}
    ${arrow(1495, 325, 1360, 465, zh ? "检查" : "inspect", c.red, "red", -60)}
    ${diamond(765, 720, 270, 150, zh ? "异常？" : "Anomaly?", zh ? "mismatch / fatal / stopped" : "mismatch / fatal / stopped", c.red)}
    ${arrow(440, 615, 765, 795, zh ? "最新状态" : "latest state", c.slate, "slate", 140)}
    ${arrow(900, 615, 900, 720, "", c.slate, "slate")}
    ${arrow(1360, 615, 1035, 795, zh ? "日志信号" : "log signal", c.slate, "slate", -140)}
    ${box({ x: 200, y: 865, w: 430, h: 130, title: zh ? "正常路径" : "Normal Path", body: [zh ? "继续同步、逐块 PASS、checkpoint 单调前进" : "continue sync, per-block PASS, checkpoint advances monotonically"], iconName: "monitor", accent: c.green, fill: c.greenSoft, compact: true, max: 25 })}
    ${box({ x: 1170, y: 840, w: 470, h: 170, title: zh ? "自动恢复路径" : "Automatic Recovery Path", body: [zh ? "归档坏库与日志、分析根因、修复、部署 dylib、重启验证" : "archive bad DB and logs, analyze root cause, fix, deploy dylib, restart validation"], iconName: "shield", accent: c.red, fill: c.redSoft, compact: true, max: 28 })}
    ${arrow(765, 795, 630, 925, zh ? "否" : "no", c.green, "green", -80)}
    ${arrow(1035, 795, 1170, 925, zh ? "是" : "yes", c.red, "red", 80)}
  `;
  return svg(t.title7, t.sub7, c.red, body);
}

const diagrams = [
  ["01-system-architecture.svg", "title1", render1],
  ["02-execution-routing.svg", "title2", render2],
  ["03-neovm-inside-riscv.svg", "title3", render3],
  ["04-syscall-native-data-flow.svg", "title4", render4],
  ["05-abi-memory-model.svg", "title5", render5],
  ["06-contract-deployment-workflow.svg", "title6", render6],
  ["07-mainnet-stateroot-validation.svg", "title7", render7],
];

function readme() {
  const row = (lang, file, key) => `| ${sets[lang][key]} | ![${sets[lang][key]}](./${lang}/${file}) |`;
  return `# Neo RISC-V VM Diagrams / Neo RISC-V VM 图解

${sets.en.intro}

${sets.zh.intro}

Regenerate all diagrams:

\`\`\`bash
node docs/diagrams/generate-diagrams.mjs
\`\`\`

## English Set

${sets.en.generated}

| Diagram | Image |
| --- | --- |
${diagrams.map(([file, key]) => row("en", file, key)).join("\n")}

## 中文图集

${sets.zh.generated}

| 图 | 图片 |
| --- | --- |
${diagrams.map(([file, key]) => row("zh", file, key)).join("\n")}
`;
}

for (const lang of ["en", "zh"]) {
  mkdirSync(path.join(root, lang), { recursive: true });
  for (const [file, , render] of diagrams) {
    writeFileSync(path.join(root, lang, file), render(lang), "utf8");
  }
}

writeFileSync(path.join(root, "README.md"), readme(), "utf8");
console.log(`Generated ${diagrams.length * 2} redesigned SVG diagrams and docs/diagrams/README.md`);
