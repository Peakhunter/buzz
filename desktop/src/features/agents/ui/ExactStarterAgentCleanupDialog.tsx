import * as React from "react";
import { ShieldAlert, Trash2 } from "lucide-react";

import type { ManagedAgent } from "@/shared/api/types";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { normalizePubkey } from "@/shared/lib/pubkey";
import {
  canConfirmExactManagedAgentDeletion,
  getStarterCleanupEntries,
} from "./exactStarterAgentCleanup";

type ExactStarterAgentCleanupDialogProps = {
  activeRelayUrl: string | null;
  agents: readonly ManagedAgent[];
  channelsByPubkey: Record<
    string,
    { id: string; name: string; channelType?: "stream" | "forum" | "dm" }[]
  >;
  isPending: boolean;
  onDeleteExact: (pubkey: string) => Promise<boolean>;
  onOpenChange: (open: boolean) => void;
  open: boolean;
};

export function ExactStarterAgentCleanupDialog({
  activeRelayUrl,
  agents,
  channelsByPubkey,
  isPending,
  onDeleteExact,
  onOpenChange,
  open,
}: ExactStarterAgentCleanupDialogProps) {
  const entries = React.useMemo(
    () => getStarterCleanupEntries(agents),
    [agents],
  );
  const [selectedPubkey, setSelectedPubkey] = React.useState<string | null>(
    null,
  );
  const [confirmation, setConfirmation] = React.useState("");
  const [isDeleting, setIsDeleting] = React.useState(false);
  const selected =
    entries.find((entry) => entry.pubkey === selectedPubkey) ?? null;
  const selectedMemberships = selected
    ? (channelsByPubkey[normalizePubkey(selected.pubkey)] ?? [])
    : [];
  const selectedDmMemberships = selectedMemberships.filter(
    (channel) => channel.channelType === "dm",
  );
  const selectedMutableMemberships = selectedMemberships.filter(
    (channel) => channel.channelType !== "dm",
  );

  React.useEffect(() => {
    if (!open) {
      setSelectedPubkey(null);
      setConfirmation("");
      setIsDeleting(false);
    }
  }, [open]);

  React.useEffect(() => {
    if (selectedPubkey && !selected) {
      setSelectedPubkey(null);
      setConfirmation("");
    }
  }, [selected, selectedPubkey]);

  const canDelete =
    selected !== null &&
    activeRelayUrl !== null &&
    selected.cleanupBlockedReason === null &&
    canConfirmExactManagedAgentDeletion(confirmation, selected.pubkey) &&
    !isDeleting &&
    !isPending;

  async function deleteSelected() {
    if (!selected || !canDelete) return;
    setIsDeleting(true);
    const removed = await onDeleteExact(selected.pubkey);
    setIsDeleting(false);
    if (removed) {
      setSelectedPubkey(null);
      setConfirmation("");
    }
  }

  const cleanupBusy = isDeleting || isPending;

  function handleOpenChange(nextOpen: boolean) {
    if (!nextOpen && cleanupBusy) return;
    onOpenChange(nextOpen);
  }

  return (
    <Dialog onOpenChange={handleOpenChange} open={open}>
      <DialogContent
        aria-describedby={undefined}
        className="max-w-3xl"
        data-testid="exact-starter-cleanup-dialog"
        onEscapeKeyDown={(event) => {
          if (cleanupBusy) event.preventDefault();
        }}
        onInteractOutside={(event) => {
          if (cleanupBusy) event.preventDefault();
        }}
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ShieldAlert className="h-5 w-5 text-destructive" />
            Exact starter identity cleanup
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4 text-sm">
          <div className="rounded-md border border-destructive/40 bg-destructive/5 p-3">
            This tool lists only instantiated local Fizz, Honey, and Bumble
            identities. It never selects by name. Every deletion is bound to the
            complete public key shown below and queues archival on the current
            relay. Persona definitions are not listed or deleted.
          </div>
          <div className="rounded-md border bg-muted/30 p-3">
            <div className="font-medium">Current community relay</div>
            <div className="mt-1 break-all font-mono text-xs">
              {activeRelayUrl ?? "No active community"}
            </div>
            <div className="mt-2 text-xs text-muted-foreground">
              Switch to the identity&apos;s configured community before deleting
              it when that community still exists. Mutable channel memberships
              on the current community are removed first; any failure stops
              deletion. DM participation is preserved as signed history and
              hidden by identity archival instead.
            </div>
            {!activeRelayUrl ? (
              <div className="mt-2 text-xs text-destructive">
                Select an active community before cleanup. Deletion remains
                disabled until memberships and archival have a relay scope.
              </div>
            ) : null}
          </div>

          {entries.length === 0 ? (
            <div
              className="rounded-md border border-emerald-500/40 bg-emerald-500/5 p-4 text-center"
              data-testid="exact-starter-cleanup-empty"
            >
              No instantiated starter identities remain.
            </div>
          ) : (
            <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
              <div className="max-h-[24rem] space-y-2 overflow-y-auto pr-1">
                {entries.map((entry) => {
                  const isSelected = entry.pubkey === selectedPubkey;
                  return (
                    <button
                      aria-pressed={isSelected}
                      className={`w-full rounded-md border p-3 text-left transition-colors ${
                        isSelected
                          ? "border-primary bg-primary/5"
                          : "hover:bg-muted/50"
                      }`}
                      data-testid={`starter-cleanup-entry-${entry.pubkey}`}
                      disabled={cleanupBusy}
                      key={entry.pubkey}
                      onClick={() => {
                        setSelectedPubkey(entry.pubkey);
                        setConfirmation("");
                      }}
                      type="button"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-medium">{entry.name}</span>
                        <span className="text-xs text-muted-foreground">
                          {entry.status}
                        </span>
                      </div>
                      <div className="mt-1 break-all font-mono text-xs leading-4">
                        {entry.pubkey}
                      </div>
                      <div className="mt-2 break-all text-xs text-muted-foreground">
                        {entry.relayUrl}
                      </div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        Created {new Date(entry.createdAt).toLocaleString()}
                      </div>
                    </button>
                  );
                })}
              </div>

              <div className="rounded-md border p-4">
                {selected ? (
                  <div className="space-y-4">
                    <div>
                      <div className="font-medium">
                        Delete exact {selected.name} identity
                      </div>
                      <div className="mt-1 break-all font-mono text-xs">
                        {selected.pubkey}
                      </div>
                    </div>
                    <div className="text-xs text-muted-foreground">
                      Removable channel memberships in the current community:{" "}
                      <strong>{selectedMutableMemberships.length}</strong>
                      {selectedMutableMemberships.length > 0
                        ? ` (${selectedMutableMemberships.map((channel) => channel.name).join(", ")})`
                        : ""}
                    </div>
                    {selectedDmMemberships.length > 0 ? (
                      <div className="text-xs text-muted-foreground">
                        Preserved DM history:{" "}
                        <strong>{selectedDmMemberships.length}</strong>
                        {` (${selectedDmMemberships.map((channel) => channel.name).join(", ")})`}
                      </div>
                    ) : null}
                    {selected.cleanupBlockedReason ? (
                      <div className="rounded-md border border-destructive/40 bg-destructive/5 p-3 text-destructive">
                        {selected.cleanupBlockedReason}
                      </div>
                    ) : (
                      <div className="space-y-2">
                        <label
                          className="text-sm font-medium leading-none"
                          htmlFor="exact-starter-delete-confirmation"
                        >
                          Type the complete public key to confirm
                        </label>
                        <Input
                          autoComplete="off"
                          data-testid="exact-starter-delete-confirmation"
                          id="exact-starter-delete-confirmation"
                          onChange={(event) =>
                            setConfirmation(event.currentTarget.value)
                          }
                          spellCheck={false}
                          value={confirmation}
                        />
                      </div>
                    )}
                    <Button
                      data-testid="exact-starter-delete-button"
                      disabled={!canDelete}
                      onClick={() => void deleteSelected()}
                      type="button"
                      variant="destructive"
                    >
                      <Trash2 />
                      {isDeleting
                        ? "Deleting exact identity…"
                        : "Delete exact identity"}
                    </Button>
                  </div>
                ) : (
                  <div className="text-muted-foreground">
                    Select one exact identity to review its full public key and
                    memberships.
                  </div>
                )}
              </div>
            </div>
          )}

          <div className="flex justify-end">
            <DialogClose asChild>
              <Button disabled={cleanupBusy} type="button" variant="outline">
                Close
              </Button>
            </DialogClose>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
