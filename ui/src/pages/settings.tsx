import { useState, useEffect } from "react";
import { Wrench, FloppyDisk } from "@phosphor-icons/react";
import { useApi } from "../hooks/use-api";
import { useToast } from "../hooks/use-toast";
import { configApi, agentsApi } from "../api";
import type { AppConfig, SystemStatus, AgentProfile } from "../types";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Select } from "../components/ui/select";
import { Skeleton } from "../components/ui/skeleton";
import { formatBytes, timeAgo } from "../lib/format";
import { cn } from "../lib/cn";

interface ConfigForm {
  allow_insecure_http: boolean;
  default_timeout_seconds: number;
  backup_count: number;
  log_retention_days: number;
  archive_after_hours: number;
  default_agent: string;
}

export default function SettingsPage() {
  const { addToast } = useToast();

  const {
    data: config,
    loading: configLoading,
    refetch: refetchConfig,
  } = useApi<AppConfig>(configApi.get);

  const { data: status, loading: statusLoading } = useApi<SystemStatus>(
    configApi.status
  );

  const { data: agents } = useApi<AgentProfile[]>(agentsApi.list);

  const [form, setForm] = useState<ConfigForm | null>(null);
  const [saving, setSaving] = useState(false);
  const [repairing, setRepairing] = useState(false);

  useEffect(() => {
    if (config && !form) {
      setForm({
        allow_insecure_http: config.allow_insecure_http ?? false,
        default_timeout_seconds: config.default_timeout_seconds ?? 300,
        backup_count: config.backup_count ?? 3,
        log_retention_days: config.log_retention_days ?? 30,
        archive_after_hours: config.archive_after_hours ?? 0,
        default_agent: config.default_agent ?? "",
      });
    }
  }, [config, form]);

  function updateForm(patch: Partial<ConfigForm>) {
    setForm((prev) => (prev ? { ...prev, ...patch } : null));
  }

  function getChangedFields(): Partial<ConfigForm> | null {
    if (!form || !config) return null;

    const changes: Partial<ConfigForm> = {};
    let hasChanges = false;

    if (form.allow_insecure_http !== (config.allow_insecure_http ?? false)) {
      changes.allow_insecure_http = form.allow_insecure_http;
      hasChanges = true;
    }
    if (form.default_timeout_seconds !== (config.default_timeout_seconds ?? 300)) {
      changes.default_timeout_seconds = form.default_timeout_seconds;
      hasChanges = true;
    }
    if (form.backup_count !== (config.backup_count ?? 3)) {
      changes.backup_count = form.backup_count;
      hasChanges = true;
    }
    if (form.log_retention_days !== (config.log_retention_days ?? 30)) {
      changes.log_retention_days = form.log_retention_days;
      hasChanges = true;
    }
    if (form.archive_after_hours !== (config.archive_after_hours ?? 0)) {
      changes.archive_after_hours = form.archive_after_hours;
      hasChanges = true;
    }
    if (form.default_agent !== (config.default_agent ?? "")) {
      changes.default_agent = form.default_agent;
      hasChanges = true;
    }

    return hasChanges ? changes : null;
  }

  async function handleSave() {
    const changes = getChangedFields();
    if (!changes) {
      addToast("info", "No changes to save");
      return;
    }

    setSaving(true);
    try {
      await configApi.update(changes);
      addToast("success", "Configuration saved");
      refetchConfig();
      setForm(null);
    } catch (err: any) {
      addToast("error", err.message || "Failed to save configuration");
    } finally {
      setSaving(false);
    }
  }

  async function handleRepair() {
    setRepairing(true);
    try {
      const result = await configApi.repair();
      addToast("success", result?.message || "Backend repair completed");
    } catch (err: any) {
      addToast("error", err.message || "Repair failed");
    } finally {
      setRepairing(false);
    }
  }

  const hasChanges = getChangedFields() !== null;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-text-primary">Settings</h1>
        <p className="mt-1 text-sm text-text-secondary">
          Manage application configuration and view system information.
        </p>
      </div>

      {/* Configuration */}
      <div className="bg-bg-panel border border-border rounded-lg p-5 space-y-5">
        <h2 className="text-lg font-medium text-text-primary">
          Configuration
        </h2>

        {configLoading || !form ? (
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-16" />
            ))}
          </div>
        ) : (
          <>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div className="flex flex-col gap-1.5">
                <label className="text-sm font-medium text-text-secondary">
                  Allow insecure HTTP
                </label>
                <button
                  type="button"
                  role="switch"
                  aria-checked={form.allow_insecure_http}
                  onClick={() =>
                    updateForm({
                      allow_insecure_http: !form.allow_insecure_http,
                    })
                  }
                  className={cn(
                    "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
                    form.allow_insecure_http ? "bg-accent" : "bg-bg-muted"
                  )}
                >
                  <span
                    className={cn(
                      "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow-sm transition-transform",
                      form.allow_insecure_http
                        ? "translate-x-5"
                        : "translate-x-0"
                    )}
                  />
                </button>
              </div>

              <Input
                label="Default timeout (seconds)"
                type="number"
                min={0}
                value={form.default_timeout_seconds}
                onChange={(e) =>
                  updateForm({
                    default_timeout_seconds: parseInt(e.target.value) || 0,
                  })
                }
              />

              <Input
                label="Backup count"
                type="number"
                min={0}
                value={form.backup_count}
                onChange={(e) =>
                  updateForm({
                    backup_count: parseInt(e.target.value) || 0,
                  })
                }
              />

              <Input
                label="Log retention (days)"
                type="number"
                min={1}
                value={form.log_retention_days}
                onChange={(e) =>
                  updateForm({
                    log_retention_days: parseInt(e.target.value) || 1,
                  })
                }
              />

              <Input
                label="Archive after hours (0 = disabled)"
                type="number"
                min={0}
                value={form.archive_after_hours}
                onChange={(e) =>
                  updateForm({
                    archive_after_hours: parseInt(e.target.value) || 0,
                  })
                }
              />

              <Select
                label="Default agent"
                value={form.default_agent}
                onChange={(e) =>
                  updateForm({ default_agent: e.target.value })
                }
              >
                <option value="">None</option>
                {agents?.map((agent) => (
                  <option key={agent.name} value={agent.name}>
                    {agent.name}
                    {agent.is_default ? " (current default)" : ""}
                  </option>
                ))}
              </Select>
            </div>

            <div className="flex justify-end">
              <Button onClick={handleSave} disabled={saving || !hasChanges}>
                <FloppyDisk size={16} weight="bold" />
                {saving ? "Saving..." : "Save Changes"}
              </Button>
            </div>
          </>
        )}
      </div>

      {/* System Information */}
      <div className="bg-bg-panel border border-border rounded-lg p-5 space-y-5">
        <h2 className="text-lg font-medium text-text-primary">System</h2>

        {statusLoading || !status ? (
          <div className="space-y-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-6" />
            ))}
          </div>
        ) : (
          <dl className="space-y-3 text-sm">
            <div className="flex items-center justify-between">
              <dt className="text-text-secondary">Backend</dt>
              <dd className="flex items-center gap-2 text-text-primary">
                <span>{status.backend}</span>
                <span
                  className={cn(
                    "inline-block h-2 w-2 rounded-full",
                    status.backend_active ? "bg-green-500" : "bg-red-500"
                  )}
                  title={status.backend_active ? "Active" : "Inactive"}
                />
              </dd>
            </div>

            <div className="flex items-center justify-between">
              <dt className="text-text-secondary">Version</dt>
              <dd className="text-text-primary">{status.version}</dd>
            </div>

            <div className="flex items-center justify-between">
              <dt className="text-text-secondary">Data Directory</dt>
              <dd className="text-text-primary font-mono text-xs">
                {status.schedx_home}
              </dd>
            </div>

            <div className="flex items-center justify-between">
              <dt className="text-text-secondary">Data Size</dt>
              <dd className="text-text-primary">
                {formatBytes(status.data_size_bytes)}
              </dd>
            </div>

            <div className="flex items-center justify-between">
              <dt className="text-text-secondary">OS / Platform</dt>
              <dd className="text-text-primary">
                {status.os} / {status.platform}
              </dd>
            </div>

            <div className="flex items-center justify-between">
              <dt className="text-text-secondary">Last Dispatch</dt>
              <dd className="text-text-primary">
                {status.last_dispatch_at ? timeAgo(status.last_dispatch_at) : "N/A"}
              </dd>
            </div>
          </dl>
        )}

        <div className="flex justify-end pt-2 border-t border-border">
          <Button
            variant="secondary"
            onClick={handleRepair}
            disabled={repairing}
          >
            <Wrench size={16} />
            {repairing ? "Repairing..." : "Repair Backend"}
          </Button>
        </div>
      </div>
    </div>
  );
}
