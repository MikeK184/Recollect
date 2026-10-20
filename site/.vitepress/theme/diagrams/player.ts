// A small story player for the explanatory diagrams. The markup is complete at
// rest; while a diagram is on screen the player walks its steps: elements with a
// matching data-key take `is-on` (live) or `is-done` (completed), slots change
// text, and one packet travels an edge per illustrated event. It stops when the
// diagram leaves the viewport or the tab is hidden, offers a pause control, and
// under reduced motion shows each step's end state without packets.

export type Packet = { path: string; reverse?: boolean; delay?: number; duration?: number };
export type Beat = { t: number; on?: string; off?: string; done?: string; flag?: string; unflag?: string; slot?: string; text?: string };
export type Step = { id: string; label: string; caption: string; duration: number; on?: string[]; done?: string[]; packets?: Packet[]; beats?: Beat[] };
export type Story = {
  id: string;
  label: string;
  rest: string;
  steps: Step[];
  restDone?: string[];
  reset?: Record<string, string>;
  final?: Record<string, string>;
  hold?: number;
};

const SVG = 'http://www.w3.org/2000/svg';
const ease = (t: number) => 1 - Math.pow(1 - t, 3);

export function storyControls(story: Story): string {
  const chips = story.steps
    .map((step, index) => `<button type="button" class="rc-chip" data-goto="${index}"><span class="rc-chip-index">${String(index + 1).padStart(2, '0')}</span>${step.label}<span class="rc-chip-progress" aria-hidden="true"></span></button>`)
    .join('');
  return `<div class="rc-story-bar">
    <div class="rc-chips" role="group" aria-label="${story.label} steps">${chips}</div>
    <button type="button" class="rc-toggle" data-toggle aria-label="Pause animation"><span class="rc-toggle-pause">${pauseIcon}</span><span class="rc-toggle-play">${playIcon}</span></button>
  </div>
  <p class="rc-caption" data-caption aria-live="polite">${story.rest}</p>`;
}

const pauseIcon = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><rect x="14" y="4" width="4" height="16" rx="1"/><rect x="6" y="4" width="4" height="16" rx="1"/></svg>';
const playIcon = '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><polygon points="6 3 20 12 6 21 6 3"/></svg>';

export function mountStory(root: HTMLElement, story: Story): () => void {
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const caption = root.querySelector<HTMLElement>('[data-caption]');
  const toggle = root.querySelector<HTMLButtonElement>('[data-toggle]');
  const chips = [...root.querySelectorAll<HTMLButtonElement>('[data-goto]')];
  let index = -1;
  let timers: number[] = [];
  let frames: number[] = [];
  let packets: SVGElement[] = [];
  let visible = false;
  let userPaused = false;
  let on = new Set<string>();
  let done = new Set<string>();
  let flags = new Set<string>();

  const keyed = (key: string) => root.querySelectorAll<Element>(`[data-key~="${key}"]`);
  function paint() {
    root.querySelectorAll('[data-key]').forEach(element => {
      const keys = (element.getAttribute('data-key') ?? '').split(/\s+/);
      element.classList.toggle('is-on', keys.some(key => on.has(key)));
      element.classList.toggle('is-done', keys.some(key => done.has(key)) && !keys.some(key => on.has(key)));
    });
    root.dataset.flags = [...flags].join(' ');
  }
  function setSlot(name: string, text: string, roll = true) {
    root.querySelectorAll<HTMLElement | SVGElement>(`[data-slot="${name}"]`).forEach(element => {
      if (element.textContent === text) return;
      element.textContent = text;
      if (roll && !reduced.matches) {
        element.classList.remove('rc-roll');
        void (element as HTMLElement).getBoundingClientRect();
        element.classList.add('rc-roll');
      }
    });
  }
  function setSlots(values: Record<string, string> | undefined, roll = false) {
    for (const [name, text] of Object.entries(values ?? {})) setSlot(name, text, roll);
  }
  function setCaption(text: string) {
    if (!caption || caption.textContent === text) return;
    caption.textContent = text;
    if (reduced.matches) return;
    caption.animate([{ opacity: 0, transform: 'translateY(4px)' }, { opacity: 1, transform: 'none' }], { duration: 220, easing: 'cubic-bezier(0.16, 1, 0.3, 1)' });
  }
  function applyBeat(beat: Beat, roll: boolean) {
    if (beat.on) on.add(beat.on);
    if (beat.off) on.delete(beat.off);
    if (beat.done) done.add(beat.done);
    if (beat.flag) flags.add(beat.flag);
    if (beat.unflag) flags.delete(beat.unflag);
    if (beat.slot && beat.text !== undefined) setSlot(beat.slot, beat.text, roll);
    paint();
  }
  function clear() {
    timers.forEach(clearTimeout);
    frames.forEach(cancelAnimationFrame);
    packets.forEach(packet => packet.remove());
    timers = [];
    frames = [];
    packets = [];
  }
  function travel(packet: Packet) {
    const path = root.querySelector<SVGPathElement>(`path[data-path="${packet.path}"]`);
    const svg = path?.ownerSVGElement;
    if (!path || !svg) return;
    const dot = document.createElementNS(SVG, 'circle');
    dot.setAttribute('r', '4');
    dot.setAttribute('class', 'rc-packet');
    svg.appendChild(dot);
    packets.push(dot);
    const length = path.getTotalLength();
    const duration = packet.duration ?? 1100;
    const start = performance.now();
    const frame = (now: number) => {
      const progress = Math.min(1, (now - start) / duration);
      const along = ease(progress) * length;
      const point = path.getPointAtLength(packet.reverse ? length - along : along);
      dot.setAttribute('cx', String(point.x));
      dot.setAttribute('cy', String(point.y));
      dot.style.opacity = String(progress < 0.08 ? progress / 0.08 : progress > 0.92 ? (1 - progress) / 0.08 : 1);
      if (progress < 1) frames.push(requestAnimationFrame(frame));
      else { dot.remove(); packets = packets.filter(item => item !== dot); }
    };
    frames.push(requestAnimationFrame(frame));
  }
  function markChips() {
    chips.forEach((chip, chipIndex) => {
      const current = chipIndex === index;
      chip.toggleAttribute('aria-current', current);
      chip.classList.toggle('is-current', current);
      chip.classList.toggle('is-past', index >= 0 && chipIndex < index);
      const bar = chip.querySelector<HTMLElement>('.rc-chip-progress');
      if (!bar) return;
      bar.getAnimations().forEach(animation => animation.cancel());
      if (current && playing() && !reduced.matches) {
        bar.animate([{ transform: 'scaleX(0)' }, { transform: 'scaleX(1)' }], { duration: story.steps[index].duration, easing: 'linear', fill: 'forwards' });
      }
    });
  }
  function rest() {
    clear();
    index = -1;
    on = new Set();
    done = new Set(story.restDone ?? []);
    flags = new Set();
    root.dataset.step = 'rest';
    setSlots(story.final);
    paint();
    setCaption(story.rest);
    markChips();
  }
  const playing = () => visible && !userPaused && !reduced.matches && document.visibilityState === 'visible';
  function show(next: number, animate: boolean) {
    clear();
    if (index < 0 || next <= index) {
      done = new Set();
      flags = new Set();
      setSlots(story.reset);
      index = -1;
    }
    for (let earlier = index + 1; earlier < next; earlier++) {
      story.steps[earlier].done?.forEach(key => done.add(key));
      story.steps[earlier].beats?.forEach(beat => applyBeat(beat, false));
    }
    index = next;
    const step = story.steps[index];
    on = new Set(step.on ?? []);
    flags = new Set();
    root.dataset.step = step.id;
    setCaption(step.caption);
    paint();
    markChips();
    if (!animate) {
      step.beats?.forEach(beat => applyBeat(beat, false));
      step.done?.forEach(key => done.add(key));
      paint();
      return;
    }
    step.packets?.forEach(packet => timers.push(window.setTimeout(() => travel(packet), packet.delay ?? 0)));
    step.beats?.forEach(beat => timers.push(window.setTimeout(() => applyBeat(beat, true), beat.t)));
    timers.push(window.setTimeout(() => {
      step.done?.forEach(key => done.add(key));
      if (index + 1 < story.steps.length) show(index + 1, true);
      else {
        rest();
        timers.push(window.setTimeout(() => { if (playing()) show(0, true); }, story.hold ?? 1800));
      }
    }, step.duration));
  }
  function sync() {
    root.classList.toggle('is-paused', userPaused);
    toggle?.setAttribute('aria-label', userPaused ? 'Play animation' : 'Pause animation');
    if (toggle) toggle.hidden = reduced.matches;
    if (playing()) {
      if (index < 0 || !timers.length) show(Math.max(0, index), true);
    } else if (timers.length) {
      const keep = index;
      clear();
      if (keep >= 0) show(keep, false);
      markChips();
    }
  }

  chips.forEach(chip => chip.addEventListener('click', () => {
    const target = Number(chip.dataset.goto);
    show(target, playing());
  }));
  toggle?.addEventListener('click', () => { userPaused = !userPaused; sync(); });
  const observer = new IntersectionObserver(entries => {
    visible = entries.some(entry => entry.isIntersecting);
    sync();
  }, { threshold: 0.35 });
  observer.observe(root);
  const onVisibility = () => sync();
  document.addEventListener('visibilitychange', onVisibility);
  reduced.addEventListener?.('change', sync);
  rest();
  sync();

  return () => {
    clear();
    observer.disconnect();
    document.removeEventListener('visibilitychange', onVisibility);
    reduced.removeEventListener?.('change', sync);
  };
}
