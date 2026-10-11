import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const Home = () => (
  <main className="bg-background text-foreground flex flex-1 flex-col items-center justify-center gap-8 p-8 text-center">
    <div className="flex flex-col items-center gap-4">
      <h1 className="max-w-2xl text-4xl font-semibold tracking-tight">
        PumaPanel
      </h1>
      <p className="text-muted-foreground max-w-md text-lg">
        Get started by editing{" "}
        <code className="bg-muted rounded-md px-1.5 py-0.5 font-mono text-sm">
          src/app/page.tsx
        </code>
      </p>
    </div>
    <div className="flex flex-wrap items-center justify-center gap-3">
      <a
        className={cn(buttonVariants())}
        href="https://nextjs.org/docs"
        rel="noopener noreferrer"
        target="_blank"
      >
        Documentation
      </a>
      <a
        className={cn(buttonVariants({ variant: "outline" }))}
        href="https://vercel.com/new"
        rel="noopener noreferrer"
        target="_blank"
      >
        Deploy
      </a>
    </div>
  </main>
);

export default Home;
