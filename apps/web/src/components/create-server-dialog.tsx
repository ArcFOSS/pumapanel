"use client";

import { Loader2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { FormEvent } from "react";

import { Button } from "@/components/ui/button";
import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
} from "@/components/ui/combobox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ApiError, GENERIC_ERROR_MESSAGE, loadVersions } from "@/lib/api";
import type { FieldError } from "@/lib/api";
import type { PaperVersion, ServersApi, VersionsResult } from "@/lib/servers";

interface CreateServerDialogProps {
  api: ServersApi;
  /** Called after a server is created, so the caller can refresh its list. */
  onCreated: () => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
}

/**
 * Client-side mirror of the API's bounds, so obvious mistakes are caught before
 * a round trip. The backend remains authoritative for the real limits.
 */
const MIN_MEMORY_MB = 256;
const MAX_MEMORY_MB = 65_536;
const DEFAULT_MEMORY_MB = 2048;

interface FieldErrors {
  memoryMb?: string;
  name?: string;
  version?: string;
}

/** The API's field errors, split into inline fields and form-level messages. */
interface SplitFieldErrors {
  errors: FieldErrors;
  others: string[];
}

/**
 * Splits the API's per-field errors into the ones the form can show inline and
 * the rest (e.g. a `hostPort` clash the user did not pick), which are surfaced
 * as a form-level message instead of being dropped.
 */
const splitApiErrorFields = (fields: FieldError[]): SplitFieldErrors => {
  const errors: FieldErrors = {};
  const others: string[] = [];

  for (const { field, message } of fields) {
    if (field === "memoryMb" || field === "name" || field === "version") {
      errors[field] = message;
    } else {
      others.push(message);
    }
  }

  return { errors, others };
};

interface VersionCatalogueStatusProps {
  error: string | null;
  loading: boolean;
  onRetry: () => void;
  stale: boolean;
}

/**
 * The catalogue's non-blocking status line under the version field: a failure
 * with a Retry button, the in-flight message, or the stale-cache notice —
 * whichever applies first.
 */
const VersionCatalogueStatus = ({
  error,
  loading,
  onRetry,
  stale,
}: VersionCatalogueStatusProps) => {
  if (error) {
    return (
      <div className="flex items-center justify-between gap-2">
        <p className="text-destructive text-sm" role="alert">
          {error}
        </p>
        <Button
          disabled={loading}
          onClick={onRetry}
          size="sm"
          type="button"
          variant="outline"
        >
          {loading ? <Loader2 className="size-4 animate-spin" /> : null}
          Retry
        </Button>
      </div>
    );
  }

  if (loading) {
    return (
      <p className="text-muted-foreground text-sm">Loading PaperMC versions…</p>
    );
  }

  if (stale) {
    return (
      <p className="text-muted-foreground text-sm">
        PaperMC is unreachable; showing the last known versions.
      </p>
    );
  }

  return null;
};

/**
 * Modal for creating a PaperMC server.
 *
 * The version catalogue is fetched from the panel — never from PaperMC
 * directly — each time the dialog opens. Open state is controlled by the caller
 * (the URL's `action=create` search param, managed with nuqs), so the dialog is
 * shareable and closes on Escape, backdrop click, or the close button.
 */
export const CreateServerDialog = ({
  api,
  onCreated,
  onOpenChange,
  open,
}: CreateServerDialogProps) => {
  const [versions, setVersions] = useState<PaperVersion[]>([]);
  const [versionsError, setVersionsError] = useState<string | null>(null);
  const [staleVersions, setStaleVersions] = useState(false);
  const [loadingVersions, setLoadingVersions] = useState(true);

  const [name, setName] = useState("");
  const [version, setVersion] = useState("");
  const [memoryMb, setMemoryMb] = useState(String(DEFAULT_MEMORY_MB));
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // The combobox filters a stable list of plain version strings. PaperMC's
  // support flag is intentionally not surfaced here: it marks only the newest
  // release line as supported, so labelling the rest would tag nearly every
  // option "unsupported" and read as if the versions were broken.
  const versionNames = useMemo(
    () => versions.map((entry) => entry.version),
    [versions]
  );

  // Applies a catalogue response to state; shared by the open effect and the
  // Retry button so those paths cannot drift.
  const applyCatalogue = useCallback((result: VersionsResult) => {
    if (result.ok) {
      setVersions(result.data.versions);
      setStaleVersions(result.data.stale);
      setVersionsError(null);
    } else {
      setVersionsError(result.message);
    }
    setLoadingVersions(false);
  }, []);

  const reloadCatalogue = useCallback(async () => {
    applyCatalogue(await loadVersions(api));
  }, [api, applyCatalogue]);

  // Load the catalogue whenever the dialog opens, and again if the client changes.
  useEffect(() => {
    if (!open) {
      return;
    }

    let active = true;
    const load = async () => {
      const result = await loadVersions(api);
      if (active) {
        applyCatalogue(result);
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, [api, applyCatalogue, open]);

  const handleRetryVersions = () => {
    setLoadingVersions(true);
    void reloadCatalogue();
  };

  const handleOpenChange = (next: boolean) => {
    if (!next) {
      // Reset the draft so a cancelled attempt does not leak into the next one.
      setName("");
      setVersion("");
      setMemoryMb(String(DEFAULT_MEMORY_MB));
      setFieldErrors({});
      setFormError(null);
    }
    onOpenChange(next);
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmedName = name.trim();
    const errors: FieldErrors = {};
    if (trimmedName.length === 0) {
      errors.name = "Enter a name for the server.";
    }
    if (version.length === 0) {
      errors.version = "Choose a PaperMC version.";
    }
    const memory = Number(memoryMb);
    if (
      !Number.isInteger(memory) ||
      memory < MIN_MEMORY_MB ||
      memory > MAX_MEMORY_MB
    ) {
      errors.memoryMb = `Use between ${MIN_MEMORY_MB} and ${MAX_MEMORY_MB} MiB.`;
    }

    setFieldErrors(errors);
    if (Object.keys(errors).length > 0) {
      return;
    }

    setSubmitting(true);
    setFormError(null);
    try {
      await api.create({ memoryMb: memory, name: trimmedName, version });
      onCreated();
      handleOpenChange(false);
    } catch (error) {
      if (error instanceof ApiError && error.fields?.length) {
        const { errors: apiErrors, others } = splitApiErrorFields(error.fields);
        setFieldErrors(apiErrors);
        if (others.length > 0) {
          setFormError(others.join(" "));
        }
      } else {
        setFormError(
          error instanceof ApiError ? error.message : GENERIC_ERROR_MESSAGE
        );
      }
    }
    setSubmitting(false);
  };

  const busy = submitting || loadingVersions;

  return (
    <Dialog onOpenChange={handleOpenChange} open={open}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create a server</DialogTitle>
          <DialogDescription>
            Pick a PaperMC version and the panel will start it on the host.
          </DialogDescription>
        </DialogHeader>

        <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="server-name">Name</Label>
            <Input
              aria-describedby={
                fieldErrors.name ? "server-name-error" : undefined
              }
              aria-invalid={fieldErrors.name ? true : undefined}
              autoComplete="off"
              disabled={submitting}
              id="server-name"
              name="name"
              onChange={(event) => setName(event.target.value)}
              placeholder="Survival"
              value={name}
            />
            {fieldErrors.name ? (
              <p className="text-destructive text-sm" id="server-name-error">
                {fieldErrors.name}
              </p>
            ) : null}
          </div>

          <div className="flex flex-col gap-1.5">
            <Label htmlFor="server-version">Version</Label>
            <Combobox
              autoHighlight
              disabled={submitting || loadingVersions}
              items={versionNames}
              onValueChange={(next) => setVersion(next ?? "")}
              value={version === "" ? null : version}
            >
              <ComboboxInput
                aria-describedby={
                  fieldErrors.version ? "server-version-error" : undefined
                }
                aria-invalid={fieldErrors.version ? true : undefined}
                autoComplete="off"
                className="w-full"
                disabled={submitting || loadingVersions}
                id="server-version"
                name="version"
                placeholder="Search PaperMC versions…"
              />
              <ComboboxContent>
                <ComboboxEmpty>No matching versions.</ComboboxEmpty>
                <ComboboxList>
                  {(item: string) => (
                    <ComboboxItem key={item} value={item}>
                      {item}
                    </ComboboxItem>
                  )}
                </ComboboxList>
              </ComboboxContent>
            </Combobox>
            {fieldErrors.version ? (
              <p className="text-destructive text-sm" id="server-version-error">
                {fieldErrors.version}
              </p>
            ) : null}
            <VersionCatalogueStatus
              error={versionsError}
              loading={loadingVersions}
              onRetry={handleRetryVersions}
              stale={staleVersions}
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <Label htmlFor="server-memory">Memory (MiB)</Label>
            <Input
              aria-describedby={
                fieldErrors.memoryMb ? "server-memory-error" : undefined
              }
              aria-invalid={fieldErrors.memoryMb ? true : undefined}
              disabled={submitting}
              id="server-memory"
              max={MAX_MEMORY_MB}
              min={MIN_MEMORY_MB}
              name="memoryMb"
              onChange={(event) => setMemoryMb(event.target.value)}
              step={256}
              type="number"
              value={memoryMb}
            />
            {fieldErrors.memoryMb ? (
              <p className="text-destructive text-sm" id="server-memory-error">
                {fieldErrors.memoryMb}
              </p>
            ) : null}
          </div>

          {formError ? (
            <p className="text-destructive text-sm" role="alert">
              {formError}
            </p>
          ) : null}

          <DialogFooter>
            <Button
              disabled={submitting}
              onClick={() => handleOpenChange(false)}
              type="button"
              variant="outline"
            >
              Cancel
            </Button>
            <Button disabled={busy} type="submit">
              {submitting ? <Loader2 className="size-4 animate-spin" /> : null}
              {submitting ? "Creating…" : "Create server"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};
