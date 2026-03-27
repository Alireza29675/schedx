import { useState, useEffect, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import {
  Terminal,
  Robot,
  Globe,
  Plus,
  X,
  ArrowLeft,
} from "@phosphor-icons/react";
import { useApi } from "../../hooks/use-api";
import { useToast } from "../../hooks/use-toast";
import { jobsApi, configApi, agentsApi } from "../../api";
import type { AgentProfile, HttpMethod } from "../../types";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { Select } from "../../components/ui/select";
import { cn } from "../../lib/cn";

type ActionType = "command" | "prompt" | "webhook";

const HTTP_METHODS: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

interface HeaderEntry {
  key: string;
  value: string;
}

interface JobFormState {
  name: string;
  tags: string[];
  schedule: string;
  action_type: ActionType;
  // Command
  command: string;
  shell_mode: boolean;
  working_directory: string;
  // Prompt
  prompt: string;
  agent: string;
  // Webhook
  url: string;
  method: HttpMethod;
  headers: HeaderEntry[];
  body: string;
  // Advanced
  timeout: number;
}

const initialForm: JobFormState = {
  name: "",
  tags: [],
  schedule: "",
  action_type: "command",
  command: "",
  shell_mode: false,
  working_directory: "",
  prompt: "",
  agent: "",
  url: "",
  method: "POST",
  headers: [],
  body: "",
  timeout: 300,
};

export default function JobFormPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const isEdit = Boolean(id);

  const { data: agents } = useApi<AgentProfile[]>(agentsApi.list);

  const [form, setForm] = useState<JobFormState>(initialForm);
  const [tagInput, setTagInput] = useState("");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [loadingJob, setLoadingJob] = useState(isEdit);

  // Schedule validation
  const [schedulePreview, setSchedulePreview] = useState<string | null>(null);
  const [scheduleError, setScheduleError] = useState<string | null>(null);
  const [validating, setValidating] = useState(false);

  function updateForm(patch: Partial<JobFormState>) {
    setForm((prev) => ({ ...prev, ...patch }));
  }

  // Load existing job for edit mode
  useEffect(() => {
    if (!id) return;

    let cancelled = false;
    (async () => {
      try {
        const job = await jobsApi.get(id);
        if (cancelled) return;

        const action = job.action;
        const actionType: ActionType = action.type;

        const base = {
          name: job.name ?? "",
          tags: job.tags ?? [],
          schedule: job.schedule_input ?? "",
          action_type: actionType,
          timeout: job.timeout_seconds ?? 300,
          // defaults for non-active action types
          command: "",
          shell_mode: false,
          working_directory: "",
          prompt: "",
          agent: "",
          url: "",
          method: "POST" as HttpMethod,
          headers: [] as HeaderEntry[],
          body: "",
        };

        switch (action.type) {
          case "command":
            base.command = action.command ?? "";
            base.shell_mode = action.shell ?? false;
            base.working_directory = action.workdir ?? "";
            break;
          case "prompt":
            base.prompt = action.text ?? "";
            base.agent = action.agent ?? "";
            break;
          case "webhook":
            base.url = action.url ?? "";
            base.method = action.method ?? "POST";
            base.headers = (action.headers ?? []).map(([key, value]) => ({
              key,
              value,
            }));
            base.body = action.body ?? "";
            break;
        }

        setForm(base);
      } catch (err: any) {
        addToast("error", err.message || "Failed to load job");
        navigate("/jobs");
      } finally {
        if (!cancelled) setLoadingJob(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [id]);

  // Debounced schedule validation
  useEffect(() => {
    if (!form.schedule.trim()) {
      setSchedulePreview(null);
      setScheduleError(null);
      return;
    }

    setValidating(true);
    const timer = setTimeout(async () => {
      try {
        const result = await configApi.validateSchedule(form.schedule);
        setSchedulePreview(result.description || "Valid");
        setScheduleError(null);
      } catch (err: any) {
        setSchedulePreview(null);
        setScheduleError(err.message || "Invalid schedule");
      } finally {
        setValidating(false);
      }
    }, 300);

    return () => {
      clearTimeout(timer);
      setValidating(false);
    };
  }, [form.schedule]);

  function handleTagKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter" && tagInput.trim()) {
      e.preventDefault();
      const tag = tagInput.trim();
      if (!form.tags.includes(tag)) {
        updateForm({ tags: [...form.tags, tag] });
      }
      setTagInput("");
    }
  }

  function removeTag(tag: string) {
    updateForm({ tags: form.tags.filter((t) => t !== tag) });
  }

  function addHeader() {
    updateForm({ headers: [...form.headers, { key: "", value: "" }] });
  }

  function updateHeader(index: number, patch: Partial<HeaderEntry>) {
    const updated = form.headers.map((h, i) =>
      i === index ? { ...h, ...patch } : h
    );
    updateForm({ headers: updated });
  }

  function removeHeader(index: number) {
    updateForm({ headers: form.headers.filter((_, i) => i !== index) });
  }

  const validate = useCallback((): string | null => {
    if (!form.schedule.trim()) return "Schedule is required";
    if (scheduleError) return "Schedule is invalid";

    switch (form.action_type) {
      case "command":
        if (!form.command.trim()) return "Command is required";
        break;
      case "prompt":
        if (!form.prompt.trim()) return "Prompt is required";
        break;
      case "webhook":
        if (!form.url.trim()) return "URL is required";
        break;
    }

    return null;
  }, [form, scheduleError]);

  async function handleSubmit() {
    const error = validate();
    if (error) {
      addToast("error", error);
      return;
    }

    const payload: Record<string, any> = {
      schedule: form.schedule,
      timeout: form.timeout,
    };

    if (form.name.trim()) payload.name = form.name.trim();
    if (form.tags.length > 0) payload.tags = form.tags;

    switch (form.action_type) {
      case "command":
        payload.command = form.command;
        payload.shell_mode = form.shell_mode;
        if (form.working_directory.trim()) {
          payload.working_directory = form.working_directory.trim();
        }
        break;
      case "prompt":
        payload.prompt = form.prompt;
        if (form.agent) payload.agent = form.agent;
        break;
      case "webhook":
        payload.url = form.url;
        payload.method = form.method;
        if (form.headers.length > 0) {
          const headers: Record<string, string> = {};
          for (const h of form.headers) {
            if (h.key.trim()) headers[h.key.trim()] = h.value;
          }
          if (Object.keys(headers).length > 0) payload.headers = headers;
        }
        if (form.body.trim()) payload.body = form.body;
        break;
    }

    setSubmitting(true);
    try {
      if (isEdit && id) {
        await jobsApi.update(id, payload);
        addToast("success", "Job updated");
        navigate(`/jobs/${id}`);
      } else {
        const created = await jobsApi.create(payload);
        addToast("success", "Job created");
        navigate(`/jobs/${created.id}`);
      }
    } catch (err: any) {
      addToast("error", err.message || "Failed to save job");
    } finally {
      setSubmitting(false);
    }
  }

  if (loadingJob) {
    return (
      <div className="flex items-center justify-center py-20 text-text-secondary text-sm">
        Loading job...
      </div>
    );
  }

  const actionTabs: { type: ActionType; label: string; icon: typeof Terminal }[] = [
    { type: "command", label: "Command", icon: Terminal },
    { type: "prompt", label: "Prompt", icon: Robot },
    { type: "webhook", label: "Webhook", icon: Globe },
  ];

  return (
    <div className="space-y-6 max-w-3xl">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            onClick={() => navigate(isEdit && id ? `/jobs/${id}` : "/jobs")}
          >
            <ArrowLeft size={16} />
          </Button>
          <h1 className="text-2xl font-semibold text-text-primary">
            {isEdit ? "Edit Job" : "Add Job"}
          </h1>
        </div>
        <Button
          variant="secondary"
          onClick={() => navigate(isEdit && id ? `/jobs/${id}` : "/jobs")}
        >
          Cancel
        </Button>
      </div>

      <div className="bg-bg-panel border border-border rounded-lg p-6 space-y-8">
        {/* Basic Info */}
        <section className="space-y-4">
          <h2 className="text-base font-medium text-text-primary">
            Basic Info
          </h2>

          <Input
            label="Name (optional)"
            placeholder="e.g. daily-backup"
            value={form.name}
            onChange={(e) => updateForm({ name: e.target.value })}
          />

          <div>
            <label className="block text-sm font-medium text-text-secondary mb-1.5">
              Tags
            </label>
            <Input
              placeholder="Type a tag and press Enter"
              value={tagInput}
              onChange={(e) => setTagInput(e.target.value)}
              onKeyDown={handleTagKeyDown}
            />
            {form.tags.length > 0 && (
              <div className="flex flex-wrap gap-1.5 mt-2">
                {form.tags.map((tag) => (
                  <span
                    key={tag}
                    className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-bg-muted text-text-secondary text-xs"
                  >
                    {tag}
                    <button
                      type="button"
                      onClick={() => removeTag(tag)}
                      className="hover:text-text-primary"
                    >
                      <X size={12} />
                    </button>
                  </span>
                ))}
              </div>
            )}
          </div>
        </section>

        {/* Schedule */}
        <section className="space-y-3">
          <h2 className="text-base font-medium text-text-primary">Schedule</h2>

          <Input
            label="Schedule expression"
            placeholder="e.g. every 5m, 0 9 * * MON-FRI, @daily"
            value={form.schedule}
            onChange={(e) => updateForm({ schedule: e.target.value })}
          />

          {form.schedule.trim() && (
            <div className="text-xs">
              {validating ? (
                <span className="text-text-secondary">Validating...</span>
              ) : scheduleError ? (
                <span className="text-red-500">{scheduleError}</span>
              ) : schedulePreview ? (
                <span className="text-green-600">{schedulePreview}</span>
              ) : null}
            </div>
          )}
        </section>

        {/* Action Type */}
        <section className="space-y-4">
          <h2 className="text-base font-medium text-text-primary">Action</h2>

          <div className="flex border-b border-border">
            {actionTabs.map(({ type, label, icon: Icon }) => (
              <button
                key={type}
                type="button"
                disabled={isEdit && form.action_type !== type}
                onClick={() => updateForm({ action_type: type })}
                className={cn(
                  "flex items-center gap-1.5 px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
                  form.action_type === type
                    ? "border-accent text-accent"
                    : "border-transparent text-text-secondary hover:text-text-primary",
                  isEdit &&
                    form.action_type !== type &&
                    "opacity-40 cursor-not-allowed"
                )}
              >
                <Icon size={16} />
                {label}
              </button>
            ))}
          </div>

          {isEdit && (
            <p className="text-xs text-text-secondary">
              Action type cannot be changed after creation.
            </p>
          )}

          {/* Command fields */}
          {form.action_type === "command" && (
            <div className="space-y-4">
              <Input
                label="Command"
                placeholder="e.g. pg_dump mydb > /backups/daily.sql"
                value={form.command}
                onChange={(e) => updateForm({ command: e.target.value })}
                className="font-mono"
              />

              <label className="flex items-center gap-2 text-sm text-text-secondary cursor-pointer">
                <input
                  type="checkbox"
                  checked={form.shell_mode}
                  onChange={(e) =>
                    updateForm({ shell_mode: e.target.checked })
                  }
                  className="rounded border-border"
                />
                <span>
                  Shell mode{" "}
                  <span className="text-text-muted">
                    (run via sh -c; supports pipes, redirects, env vars)
                  </span>
                </span>
              </label>

              <Input
                label="Working directory (optional)"
                placeholder="/path/to/directory"
                value={form.working_directory}
                onChange={(e) =>
                  updateForm({ working_directory: e.target.value })
                }
              />
            </div>
          )}

          {/* Prompt fields */}
          {form.action_type === "prompt" && (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1.5">
                  Prompt
                </label>
                <textarea
                  rows={4}
                  placeholder="Enter the prompt to send to the agent..."
                  value={form.prompt}
                  onChange={(e) => updateForm({ prompt: e.target.value })}
                  className="w-full rounded-md border border-border bg-bg-input px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-accent/50 focus:border-accent resize-y"
                />
              </div>

              <Select
                label="Agent"
                value={form.agent}
                onChange={(e) => updateForm({ agent: e.target.value })}
              >
                <option value="">Default</option>
                {agents?.map((agent) => (
                  <option key={agent.name} value={agent.name}>
                    {agent.name}
                  </option>
                ))}
              </Select>
            </div>
          )}

          {/* Webhook fields */}
          {form.action_type === "webhook" && (
            <div className="space-y-4">
              <div>
                <Input
                  label="URL"
                  placeholder="https://example.com/webhook"
                  value={form.url}
                  onChange={(e) => updateForm({ url: e.target.value })}
                />
                {form.url.startsWith("http://") && (
                  <p className="mt-1 text-xs text-amber-500">
                    Warning: Using insecure HTTP. Consider HTTPS instead.
                  </p>
                )}
              </div>

              <Select
                label="Method"
                value={form.method}
                onChange={(e) =>
                  updateForm({ method: e.target.value as HttpMethod })
                }
              >
                {HTTP_METHODS.map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </Select>

              {/* Headers */}
              <div>
                <div className="flex items-center justify-between mb-1.5">
                  <label className="text-sm font-medium text-text-secondary">
                    Headers
                  </label>
                  <Button variant="ghost" onClick={addHeader}>
                    <Plus size={14} />
                    Add
                  </Button>
                </div>
                {form.headers.length > 0 && (
                  <div className="space-y-2">
                    {form.headers.map((header, i) => (
                      <div key={i} className="flex items-center gap-2">
                        <Input
                          placeholder="Header name"
                          value={header.key}
                          onChange={(e) =>
                            updateHeader(i, { key: e.target.value })
                          }
                          className="flex-1"
                        />
                        <Input
                          placeholder="Value"
                          value={header.value}
                          onChange={(e) =>
                            updateHeader(i, { value: e.target.value })
                          }
                          className="flex-1"
                        />
                        <Button
                          variant="ghost"
                          onClick={() => removeHeader(i)}
                          className="text-red-500 hover:text-red-600 shrink-0"
                        >
                          <X size={14} />
                        </Button>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-text-secondary mb-1.5">
                  Body
                </label>
                <textarea
                  rows={4}
                  placeholder='{"key": "value"}'
                  value={form.body}
                  onChange={(e) => updateForm({ body: e.target.value })}
                  className="w-full rounded-md border border-border bg-bg-input px-3 py-2 text-sm text-text-primary font-mono placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-accent/50 focus:border-accent resize-y"
                />
              </div>
            </div>
          )}
        </section>

        {/* Advanced */}
        <section>
          <button
            type="button"
            onClick={() => setAdvancedOpen(!advancedOpen)}
            className="text-sm font-medium text-text-secondary hover:text-text-primary transition-colors"
          >
            {advancedOpen ? "Hide" : "Show"} advanced options
          </button>

          {advancedOpen && (
            <div className="mt-3 space-y-4">
              <Input
                label="Timeout (seconds)"
                type="number"
                min={0}
                value={form.timeout}
                onChange={(e) =>
                  updateForm({ timeout: parseInt(e.target.value) || 0 })
                }
              />
            </div>
          )}
        </section>

        {/* Buttons */}
        <div className="flex items-center gap-3 pt-4 border-t border-border">
          <Button onClick={handleSubmit} disabled={submitting}>
            {submitting
              ? "Saving..."
              : isEdit
                ? "Save Changes"
                : "Create Job"}
          </Button>
          <Button
            variant="secondary"
            onClick={() => navigate(isEdit && id ? `/jobs/${id}` : "/jobs")}
          >
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
}
