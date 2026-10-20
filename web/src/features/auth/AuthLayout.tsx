import type { ReactNode } from "react";
import { CircleCheck, FileCog, NotebookText, UserRound } from "lucide-react";
import { Brand } from "../../components/Brand";
import { staggerStyle } from "../../components/Motion";
import { Tile } from "../../components/ui";

// Full-screen layout shared by sign-in and invitation enrollment: a dark story
// panel (always Ink, whatever the theme) beside a centred 380px form column.
const stages = [
  { icon: UserRound, label: "Contributed", detail: "Agents and people" },
  { icon: FileCog, label: "Processing", detail: "Exact source version" },
  { icon: NotebookText, label: "Memory", detail: "With its evidence" },
] as const;

function Story() {
  return (
    <section
      data-theme="dark"
      aria-label="About Recollect"
      className="m-2 hidden flex-col justify-between gap-12 rounded-lg border border-terminal-border bg-terminal px-14 py-12 text-terminal-foreground lg:flex"
    >
      <div>
        <Brand size="lg" />
      </div>
      <div className="flex flex-col gap-5">
        <p className="m-0 section-label text-terminal-muted">
          Your engineering memory
        </p>
        <p className="m-0 text-balance font-serif text-[48px] leading-[54px] tracking-[-0.02em] text-terminal-foreground">
          Keep the context.
          <br />
          Build on what you know.
        </p>
        <p className="m-0 max-w-[420px] text-[16px] leading-[26px] text-terminal-foreground">
          A home for your team’s knowledge, decisions and the evidence behind
          them.
        </p>
        <div
          className="mt-7 flex max-w-[560px] items-center"
          aria-hidden="true"
        >
          {stages.map(({ icon: Icon, label, detail }, index) => (
            <div key={label} className="contents">
              {index > 0 && (
                <span className="min-w-6 flex-[1_1_24px] border-t-[1.5px] border-brand/50" />
              )}
              <div
                className="grid w-[150px] flex-none animate-enter gap-2 rounded-lg border border-border bg-card p-3 shadow-xs"
                style={staggerStyle(index + 1)}
              >
                <div className="flex items-center gap-2">
                  <Tile tone="brand" size="sm">
                    <Icon />
                  </Tile>
                  <CircleCheck className="ml-auto size-4 text-brand-text" />
                </div>
                <span className="section-label">{label}</span>
                <p className="m-0 -mt-1.5 text-caption text-muted-foreground">
                  {detail}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
      <p className="m-0 text-body text-terminal-muted">
        Your knowledge. Your infrastructure.
      </p>
    </section>
  );
}

/** Sign-in and enrollment frame; `children` is the form column's content. */
export function AuthLayout({
  labelledBy,
  children,
}: {
  /** Id of the form column's h1. */
  labelledBy: string;
  children: ReactNode;
}) {
  return (
    <div className="grid min-h-svh bg-background text-body text-foreground lg:grid-cols-[minmax(0,44fr)_minmax(0,56fr)]">
      <Story />
      <main className="grid place-items-center px-6 py-12">
        <section
          aria-labelledby={labelledBy}
          className="flex w-[380px] max-w-full animate-enter flex-col gap-6"
        >
          {children}
        </section>
      </main>
    </div>
  );
}

/** Badge, h1 and one-line description at the top of the form column. */
export function AuthHeading({
  id,
  badge,
  title,
  description,
}: {
  id: string;
  badge?: ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="flex flex-col items-start gap-3.5">
      {badge}
      <div>
        <h1 id={id} className="m-0 text-balance text-page font-semibold">
          {title}
        </h1>
        <p className="m-0 mt-1 text-body text-muted-foreground">
          {description}
        </p>
      </div>
    </div>
  );
}
