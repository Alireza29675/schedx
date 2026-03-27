import { useState } from "react";
import { Plus, Trash, Star, PencilSimple, Cpu } from "@phosphor-icons/react";
import { useApi } from "../hooks/use-api";
import { useToast } from "../hooks/use-toast";
import { agentsApi } from "../api";
import type { AgentProfile } from "../types";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { ConfirmDialog } from "../components/ui/confirm-dialog";
import { EmptyState } from "../components/ui/empty-state";
import { TableSkeleton } from "../components/ui/skeleton";

interface AgentFormData {
  name: string;
  bin: string;
  args: string;
  prompt_stdin: boolean;
}

const emptyForm: AgentFormData = {
  name: "",
  bin: "",
  args: "",
  prompt_stdin: false,
};

export default function AgentsPage() {
  const { addToast } = useToast();
  const {
    data: agents,
    loading,
    refetch,
  } = useApi<AgentProfile[]>(agentsApi.list);

  const [formOpen, setFormOpen] = useState(false);
  const [form, setForm] = useState<AgentFormData>(emptyForm);
  const [creating, setCreating] = useState(false);

  const [editingName, setEditingName] = useState<string | null>(null);
  const [editForm, setEditForm] = useState<AgentFormData>(emptyForm);
  const [saving, setSaving] = useState(false);

  const [confirmRemove, setConfirmRemove] = useState<AgentProfile | null>(null);
  const [, setRemoving] = useState(false);

  function updateForm(patch: Partial<AgentFormData>) {
    setForm((prev) => ({ ...prev, ...patch }));
  }

  function updateEditForm(patch: Partial<AgentFormData>) {
    setEditForm((prev) => ({ ...prev, ...patch }));
  }

  async function handleCreate() {
    if (!form.name.trim() || !form.bin.trim()) {
      addToast("error", "Name and binary path are required");
      return;
    }

    setCreating(true);
    try {
      const args = form.args
        .trim()
        .split(/\s+/)
        .filter(Boolean);

      await agentsApi.create({
        name: form.name.trim(),
        bin: form.bin.trim(),
        args: args.length > 0 ? args : [],
        prompt_stdin: form.prompt_stdin,
      });

      addToast("success", `Agent "${form.name}" created`);
      setForm(emptyForm);
      setFormOpen(false);
      refetch();
    } catch (err: any) {
      addToast("error", err.message || "Failed to create agent");
    } finally {
      setCreating(false);
    }
  }

  function startEdit(agent: AgentProfile) {
    setEditingName(agent.name);
    setEditForm({
      name: agent.name,
      bin: agent.bin,
      args: agent.args?.join(" ") ?? "",
      prompt_stdin: agent.prompt_stdin,
    });
  }

  function cancelEdit() {
    setEditingName(null);
    setEditForm(emptyForm);
  }

  async function handleSaveEdit() {
    if (!editingName) return;
    if (!editForm.bin.trim()) {
      addToast("error", "Binary path is required");
      return;
    }

    setSaving(true);
    try {
      const args = editForm.args
        .trim()
        .split(/\s+/)
        .filter(Boolean);

      await agentsApi.update(editingName, {
        bin: editForm.bin.trim(),
        args: args.length > 0 ? args : undefined,
        prompt_stdin: editForm.prompt_stdin,
      });

      addToast("success", `Agent "${editingName}" updated`);
      cancelEdit();
      refetch();
    } catch (err: any) {
      addToast("error", err.message || "Failed to update agent");
    } finally {
      setSaving(false);
    }
  }

  async function handleSetDefault(name: string) {
    try {
      await agentsApi.setDefault(name);
      addToast("success", `"${name}" is now the default agent`);
      refetch();
    } catch (err: any) {
      addToast("error", err.message || "Failed to set default agent");
    }
  }

  async function handleRemove() {
    if (!confirmRemove) return;

    setRemoving(true);
    try {
      await agentsApi.remove(confirmRemove.name);
      addToast("success", `Agent "${confirmRemove.name}" removed`);
      setConfirmRemove(null);
      refetch();
    } catch (err: any) {
      addToast("error", err.message || "Failed to remove agent");
    } finally {
      setRemoving(false);
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-text-primary">Agents</h1>
          <p className="mt-1 text-sm text-text-secondary">
            Manage agent profiles used to execute prompt-based jobs.
          </p>
        </div>
        <Button
          variant={formOpen ? "secondary" : "primary"}
          onClick={() => setFormOpen(!formOpen)}
        >
          <Plus size={16} weight="bold" />
          {formOpen ? "Cancel" : "Add Agent"}
        </Button>
      </div>

      {formOpen && (
        <div className="bg-bg-panel border border-border rounded-lg p-5 space-y-4">
          <h2 className="text-lg font-medium text-text-primary">New Agent</h2>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <Input
              label="Name"
              placeholder="e.g. claude-code"
              value={form.name}
              onChange={(e) => updateForm({ name: e.target.value })}
              required
            />
            <Input
              label="Binary Path"
              placeholder="e.g. /usr/local/bin/claude"
              value={form.bin}
              onChange={(e) => updateForm({ bin: e.target.value })}
              required
            />
          </div>

          <Input
            label="Arguments"
            placeholder="Space-separated arguments, e.g. --model opus --verbose"
            value={form.args}
            onChange={(e) => updateForm({ args: e.target.value })}
          />

          <label className="flex items-center gap-2 text-sm text-text-secondary cursor-pointer">
            <input
              type="checkbox"
              checked={form.prompt_stdin}
              onChange={(e) =>
                updateForm({ prompt_stdin: e.target.checked })
              }
              className="rounded border-border"
            />
            Prompt via stdin
          </label>

          <div className="flex justify-end">
            <Button onClick={handleCreate} disabled={creating}>
              {creating ? "Creating..." : "Create Agent"}
            </Button>
          </div>
        </div>
      )}

      <div className="bg-bg-panel border border-border rounded-lg overflow-hidden">
        {loading ? (
          <TableSkeleton rows={4} cols={7} />
        ) : !agents || agents.length === 0 ? (
          <EmptyState
            icon={<Cpu size={48} />}
            title="No agents configured"
            description="Add an agent profile to start running prompt-based jobs."
          />
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-left text-text-secondary">
                  <th className="px-4 py-3 font-medium">Name</th>
                  <th className="px-4 py-3 font-medium">Binary</th>
                  <th className="px-4 py-3 font-medium">Arguments</th>
                  <th className="px-4 py-3 font-medium">Stdin</th>
                  <th className="px-4 py-3 font-medium">Default</th>
                  <th className="px-4 py-3 font-medium">Jobs</th>
                  <th className="px-4 py-3 font-medium text-right">Actions</th>
                </tr>
              </thead>
              <tbody>
                {agents.map((agent) => (
                  <tr
                    key={agent.name}
                    className="border-b border-border last:border-0 hover:bg-bg-hover transition-colors"
                  >
                    {editingName === agent.name ? (
                      <td colSpan={7} className="px-4 py-3">
                        <div className="space-y-3">
                          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                            <Input
                              label="Binary Path"
                              value={editForm.bin}
                              onChange={(e) =>
                                updateEditForm({
                                  bin: e.target.value,
                                })
                              }
                            />
                            <Input
                              label="Arguments"
                              value={editForm.args}
                              onChange={(e) =>
                                updateEditForm({
                                  args: e.target.value,
                                })
                              }
                            />
                          </div>
                          <label className="flex items-center gap-2 text-sm text-text-secondary cursor-pointer">
                            <input
                              type="checkbox"
                              checked={editForm.prompt_stdin}
                              onChange={(e) =>
                                updateEditForm({
                                  prompt_stdin: e.target.checked,
                                })
                              }
                              className="rounded border-border"
                            />
                            Prompt via stdin
                          </label>
                          <div className="flex gap-2 justify-end">
                            <Button
                              variant="secondary"
                              onClick={cancelEdit}
                            >
                              Cancel
                            </Button>
                            <Button
                              onClick={handleSaveEdit}
                              disabled={saving}
                            >
                              {saving ? "Saving..." : "Save"}
                            </Button>
                          </div>
                        </div>
                      </td>
                    ) : (
                      <>
                        <td className="px-4 py-3 font-mono text-text-primary">
                          {agent.name}
                        </td>
                        <td className="px-4 py-3 font-mono text-text-code">
                          {agent.bin}
                        </td>
                        <td className="px-4 py-3 font-mono text-text-code">
                          {agent.args && agent.args.length > 0
                            ? agent.args.join(" ")
                            : "-"}
                        </td>
                        <td className="px-4 py-3 text-text-secondary">
                          {agent.prompt_stdin ? "Yes" : "No"}
                        </td>
                        <td className="px-4 py-3">
                          {agent.is_default && (
                            <Star
                              size={16}
                              weight="fill"
                              className="text-amber-500"
                            />
                          )}
                        </td>
                        <td className="px-4 py-3 text-text-secondary">
                          {agent.job_count}
                        </td>
                        <td className="px-4 py-3">
                          <div className="flex items-center gap-1 justify-end">
                            {!agent.is_default && (
                              <Button
                                variant="ghost"
                                onClick={() => handleSetDefault(agent.name)}
                                title="Set as default"
                              >
                                <Star size={16} />
                              </Button>
                            )}
                            <Button
                              variant="ghost"
                              onClick={() => startEdit(agent)}
                              title="Edit"
                            >
                              <PencilSimple size={16} />
                            </Button>
                            <Button
                              variant="ghost"
                              onClick={() => setConfirmRemove(agent)}
                              title="Remove"
                              className="text-red-500 hover:text-red-600"
                            >
                              <Trash size={16} />
                            </Button>
                          </div>
                        </td>
                      </>
                    )}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <ConfirmDialog
        open={confirmRemove !== null}
        onCancel={() => setConfirmRemove(null)}
        onConfirm={handleRemove}
        title="Remove Agent"
        description={
          confirmRemove && confirmRemove.job_count > 0
            ? `"${confirmRemove.name}" is used by ${confirmRemove.job_count} job(s). Removing it will leave those jobs without an agent. Continue?`
            : `Remove agent "${confirmRemove?.name}"?`
        }
        confirmLabel="Remove"
        variant="danger"
      />
    </div>
  );
}
