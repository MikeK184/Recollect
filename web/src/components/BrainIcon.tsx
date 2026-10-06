import { useState } from "react";
import { Layers3 } from "lucide-react";

export function BrainIcon({
  id,
  revision,
  size = 38,
}: {
  id: string;
  revision?: string | null;
  size?: number;
}) {
  const [failed, setFailed] = useState<string | null>(null);
  const source = revision
    ? `/api/brains/${encodeURIComponent(id)}/icon?v=${encodeURIComponent(revision)}`
    : null;
  return (
    <span
      className="brain-artwork"
      style={{ width: size, height: size }}
      aria-hidden="true"
    >
      {source && failed !== source ? (
        <img src={source} alt="" onError={() => setFailed(source)} />
      ) : (
        <Layers3 size={Math.round(size * 0.58)} />
      )}
    </span>
  );
}

export async function prepareBrainIcon(
  file: File,
): Promise<{ blob: Blob; preview: string }> {
  if (file.size > 512 * 1024)
    throw new Error("Choose an icon no larger than 512 KiB.");
  if (!/\.(png|svg|ico)$/i.test(file.name))
    throw new Error("Choose a PNG, SVG or ICO file.");
  let input: Blob = file;
  if (/\.svg$/i.test(file.name)) {
    const source = await file.text();
    if (/<!DOCTYPE|<!ENTITY/i.test(source))
      throw new Error("Use a self-contained SVG without external resources.");
    const doc = new DOMParser().parseFromString(source, "image/svg+xml");
    const root = doc.documentElement;
    const nodes = [...doc.querySelectorAll("*")];
    const allowed = new Set([
      "svg",
      "g",
      "path",
      "rect",
      "circle",
      "ellipse",
      "line",
      "polyline",
      "polygon",
      "defs",
      "linearGradient",
      "radialGradient",
      "stop",
      "clipPath",
      "mask",
      "use",
      "title",
      "desc",
      "style",
      "text",
      "tspan",
    ]);
    if (
      root.localName !== "svg" ||
      doc.querySelector("parsererror") ||
      nodes.length > 1500
    )
      throw new Error("This SVG is invalid or too complex for an icon.");
    for (const node of nodes) {
      if (!allowed.has(node.localName))
        throw new Error(
          "Use a static SVG without scripts, animation or embedded content.",
        );
      for (const attr of [...node.attributes]) {
        if (
          /^on/i.test(attr.name) ||
          (attr.localName === "href" && !attr.value.trim().startsWith("#"))
        )
          throw new Error(
            "Use a self-contained SVG without external resources.",
          );
      }
    }
    const cssValues = nodes.flatMap(node => [node.localName === "style" ? node.textContent ?? "" : "", ...[...node.attributes].map(attr => attr.value)]).join("\n");
    if (
      /@import/i.test(cssValues) ||
      [...cssValues.matchAll(/url\(([^)]*)\)/gi)].some(
        (match) =>
          !match[1]
            .trim()
            .replace(/^["']|["']$/g, "")
            .startsWith("#"),
      )
    )
      throw new Error("Use a self-contained SVG without external resources.");
    if (!root.hasAttribute("viewBox")) {
      const width = parseFloat(root.getAttribute("width") ?? "256"),
        height = parseFloat(root.getAttribute("height") ?? "256");
      if (!(width > 0 && height > 0 && Number.isFinite(width + height)))
        throw new Error("The SVG needs valid dimensions.");
      root.setAttribute("viewBox", `0 0 ${width} ${height}`);
    }
    root.setAttribute("width", "256");
    root.setAttribute("height", "256");
    input = new Blob([new XMLSerializer().serializeToString(root)], {
      type: "image/svg+xml",
    });
  }
  const url = URL.createObjectURL(input),
    image = new Image();
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    image.src = url;
    await Promise.race([
      image.decode(),
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error("This image took too long to decode.")),
          8000,
        );
      }),
    ]);
    if (
      !image.naturalWidth ||
      !image.naturalHeight ||
      image.naturalWidth > 4096 ||
      image.naturalHeight > 4096
    )
      throw new Error("Choose an image within 4096 × 4096 pixels.");
    const scale = Math.min(
      1,
      256 / Math.max(image.naturalWidth, image.naturalHeight),
    );
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
    canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
    const context = canvas.getContext("2d");
    if (!context) throw new Error("This browser could not prepare the icon.");
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise<Blob>((resolve, reject) =>
      canvas.toBlob(
        (value) =>
          value
            ? resolve(value)
            : reject(new Error("The icon could not be prepared.")),
        "image/png",
      ),
    );
    return { blob, preview: canvas.toDataURL("image/png") };
  } catch (error) {
    throw error instanceof Error
      ? error
      : new Error("This image could not be read.");
  } finally {
    clearTimeout(timer);
    URL.revokeObjectURL(url);
  }
}
