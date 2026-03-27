import { useState, useCallback, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import {
  Play,
  Pause,
  PlayCircle,
  Trash,
  Plus,
  ListDashes,
} from "@phosphor-icons/react";
import { useApi } from "../../hooks/use-api";
import { useSSE } from "../../hooks/use-sse";
import { useToast } from "../../hooks/use-toast";
import { jobsApi } from "../../api";
import type { JobListEntry } from "../../types";
import { JobStatusBadge } from "../../components/shared/job-status-badge";
import { ActionTypeIcon } from "../../components/shared/action-type-icon";
import { TimeAgo } from "../../components/shared/time-ago";
import { Tag } from "../../components/ui/tag";
import { Button } from "../../components/ui/button";
import { Select } from "../../components/ui/select";
import { EmptyState } from "../../components/ui/empty-state";
import { ConfirmDialog } from "../../components/ui/confirm-dialog";
import { TableSkeleton } from "../../components/ui/skeleton";
import { formatCountdown } from "../../lib/format";
import { cn } from "../../lib/cn";

type SortOption =
  | "name-asc"
  | "last-run-desc"
  | "next-run-asc"
  | "created-desc"
  | "run-count-desc";

const statusOptions: { value: string; label: string }[] = [
  { value: "", label: "All Statuses" },
  { value: "active", label: "Active" },
  { value: "paused", label: "Paused" },
  { value: "completed", label: "Completed" },
  { value: "archived", label: "Archived" },
];

const typeOptions: { value: string; label: string }[] = [
  { value: "", label: "All Types" },
  { value: "command", label: "Command" },
  { value: "prompt", label: "Prompt" },
  { value: "webhook", label: "Webhook" },
];

const sortOptions: { value: SortOption; label: string }[] = [
  { value: "name-asc", label: "Name (A-Z)" },
  { value: "last-run-desc", label: "Last Run (Newest)" },
  { value: "next-run-asc", label: "Next Run (Soonest)" },
  { value: "created-desc", label: "Created (Newest)" },
  { value: "run-count-desc", label: "Run Count" },
];

export default function JobsList() {
  const navigate = useNavigate();
  const { addToast } = useToast();

  const [statusFilter, setStatusFilter] = useState<string>("");
  const [typeFilter, setTypeFilter] = useState<string>("");
  const [tagFilter, setTagFilter] = useState<string>("");
  const [sort, setSort] = useState<SortOption>("created-desc");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [deleteTarget, setDeleteTarget] = useState<JobListEntry | null>(null);

  const params = useMemo(
    () => ({
      status: statusFilter || undefined,
      action_type: typeFilter || undefined,
      tag: tagFilter || undefined,
      sort,
    }),
    [statusFilter, typeFilter, tagFilter, sort]
  );

  const {
    data: jobs,
    loading,
    refetch,
  } = useApi(() => jobsApi.list(params), [params]);

  useSSE((event) => { if (event === 'job-changed') refetch() });

  const handlePause = useCallback(
    async (e: React.MouseEvent, job: JobListEntry) => {
      e.stopPropagation();
      try {
        await jobsApi.pause(job.id);
        addToast('success',`Paused "${job.name || job.id}"`);
        refetch();
      } catch {
        addToast('error',"Failed to pause job");
      }
    },
    [addToast, refetch]
  );

  const handleResume = useCallback(
    async (e: React.MouseEvent, job: JobListEntry) => {
      e.stopPropagation();
      try {
        await jobsApi.resume(job.id);
        addToast('success',`Resumed "${job.name || job.id}"`);
        refetch();
      } catch {
        addToast('error',"Failed to resume job");
      }
    },
    [addToast, refetch]
  );

  const handleRunNow = useCallback(
    async (e: React.MouseEvent, job: JobListEntry) => {
      e.stopPropagation();
      try {
        await jobsApi.run(job.id);
        addToast('success',`Triggered "${job.name || job.id}"`);
        refetch();
      } catch {
        addToast('error',"Failed to trigger job");
      }
    },
    [addToast, refetch]
  );

  const handleDeleteClick = useCallback(
    (e: React.MouseEvent, job: JobListEntry) => {
      e.stopPropagation();
      setDeleteTarget(job);
    },
    []
  );

  const handleDeleteConfirm = useCallback(async () => {
    if (!deleteTarget) return;
    try {
      await jobsApi.remove(deleteTarget.id);
      addToast('success',`Deleted "${deleteTarget.name || deleteTarget.id}"`);
      setSelectedIds((prev) => {
        const next = new Set(prev);
        next.delete(deleteTarget.id);
        return next;
      });
      setDeleteTarget(null);
      refetch();
    } catch {
      addToast('error',"Failed to delete job");
    }
  }, [deleteTarget, addToast, refetch]);

  const handleSelectAll = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      if (!jobs) return;
      if (e.target.checked) {
        setSelectedIds(new Set(jobs.map((j) => j.id)));
      } else {
        setSelectedIds(new Set());
      }
    },
    [jobs]
  );

  const handleSelectOne = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>, id: string) => {
      e.stopPropagation();
      setSelectedIds((prev) => {
        const next = new Set(prev);
        if (next.has(id)) {
          next.delete(id);
        } else {
          next.add(id);
        }
        return next;
      });
    },
    []
  );

  const allSelected =
    jobs && jobs.length > 0 && selectedIds.size === jobs.length;

  if (loading) {
    return (
      <div className="space-y-4">
        <div className="bg-bg-panel border border-border rounded-lg p-3 h-14" />
        <TableSkeleton rows={8} cols={9} />
      </div>
    );
  }

  if (!jobs || jobs.length === 0) {
    const hasFilters = statusFilter || typeFilter || tagFilter;

    if (hasFilters) {
      return (
        <div className="space-y-4">
          <FilterBar
            statusFilter={statusFilter}
            setStatusFilter={setStatusFilter}
            typeFilter={typeFilter}
            setTypeFilter={setTypeFilter}
            tagFilter={tagFilter}
            setTagFilter={setTagFilter}
            sort={sort}
            setSort={setSort}
          />
          <div className="bg-bg-panel border border-border rounded-lg">
            <EmptyState
              icon={<ListDashes size={48} />}
              title="No matching jobs"
              description="Try adjusting your filters."
            />
          </div>
        </div>
      );
    }

    return (
      <EmptyState
        icon={<ListDashes size={48} />}
        title="No jobs yet"
        description="Create your first scheduled job to get started."
        action={
          <Button onClick={() => navigate("/jobs/new")}>
            <Plus size={16} weight="bold" />
            Add Job
          </Button>
        }
      />
    );
  }

  return (
    <div className="space-y-4">
      <FilterBar
        statusFilter={statusFilter}
        setStatusFilter={setStatusFilter}
        typeFilter={typeFilter}
        setTypeFilter={setTypeFilter}
        tagFilter={tagFilter}
        setTagFilter={setTagFilter}
        sort={sort}
        setSort={setSort}
      />

      <div className="bg-bg-panel border border-border rounded-lg overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="sticky top-0 z-10 bg-bg-panel border-b border-border">
              <tr className="text-left text-text-muted">
                <th className="p-3 w-10">
                  <input
                    type="checkbox"
                    checked={!!allSelected}
                    onChange={handleSelectAll}
                    className="rounded border-border"
                  />
                </th>
                <th className="p-3 font-medium">Name</th>
                <th className="p-3 font-medium">Status</th>
                <th className="p-3 font-medium">Type</th>
                <th className="p-3 font-medium">Schedule</th>
                <th className="p-3 font-medium">Tags</th>
                <th className="p-3 font-medium">Last Run</th>
                <th className="p-3 font-medium">Next Run</th>
                <th className="p-3 font-medium w-28">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {jobs.map((job) => (
                <tr
                  key={job.id}
                  onClick={() => navigate(`/jobs/${job.id}`)}
                  className="group cursor-pointer hover:bg-bg-hover transition-colors"
                >
                  <td className="p-3" onClick={(e) => e.stopPropagation()}>
                    <input
                      type="checkbox"
                      checked={selectedIds.has(job.id)}
                      onChange={(e) => handleSelectOne(e, job.id)}
                      className="rounded border-border"
                    />
                  </td>
                  <td className="p-3">
                    <div className="font-medium text-text-primary">
                      {job.name || job.id}
                    </div>
                    {job.name && (
                      <div className="text-xs text-text-muted mt-0.5 font-mono">
                        {job.id}
                      </div>
                    )}
                  </td>
                  <td className="p-3">
                    <JobStatusBadge status={job.status} />
                  </td>
                  <td className="p-3">
                    <ActionTypeIcon type={job.action_type} />
                  </td>
                  <td className="p-3">
                    <div className="text-text-primary font-mono text-xs">
                      {job.schedule_input}
                    </div>
                  </td>
                  <td className="p-3">
                    <div className="flex flex-wrap gap-1">
                      {job.tags?.map((tag) => (
                        <Tag key={tag}>{tag}</Tag>
                      ))}
                    </div>
                  </td>
                  <td className="p-3">
                    {job.last_run?.started_at ? (
                      <div className="flex items-center gap-1.5">
                        <span
                          className={cn(
                            "w-1.5 h-1.5 rounded-full shrink-0",
                            job.last_run?.status === "success"
                              ? "bg-green-500"
                              : job.last_run?.status === "failed"
                                ? "bg-red-500"
                                : "bg-yellow-500"
                          )}
                        />
                        <TimeAgo date={job.last_run.started_at} />
                      </div>
                    ) : (
                      <span className="text-text-muted">-</span>
                    )}
                  </td>
                  <td className="p-3">
                    {job.status === "active" && job.next_run ? (
                      <span className="text-text-primary">
                        {formatCountdown(job.next_run)}
                      </span>
                    ) : (
                      <span className="text-text-muted">-</span>
                    )}
                  </td>
                  <td className="p-3">
                    <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
                        onClick={(e) => handleRunNow(e, job)}
                        className="p-1.5 rounded hover:bg-bg-active text-text-muted hover:text-text-primary transition-colors"
                        title="Run now"
                      >
                        <PlayCircle size={18} />
                      </button>
                      {job.status === "active" ? (
                        <button
                          onClick={(e) => handlePause(e, job)}
                          className="p-1.5 rounded hover:bg-bg-active text-text-muted hover:text-text-primary transition-colors"
                          title="Pause"
                        >
                          <Pause size={18} />
                        </button>
                      ) : job.status === "paused" ? (
                        <button
                          onClick={(e) => handleResume(e, job)}
                          className="p-1.5 rounded hover:bg-bg-active text-text-muted hover:text-text-primary transition-colors"
                          title="Resume"
                        >
                          <Play size={18} />
                        </button>
                      ) : null}
                      <button
                        onClick={(e) => handleDeleteClick(e, job)}
                        className="p-1.5 rounded hover:bg-red-500/10 text-text-muted hover:text-red-500 transition-colors"
                        title="Delete"
                      >
                        <Trash size={18} />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      <ConfirmDialog
        open={!!deleteTarget}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={handleDeleteConfirm}
        title="Delete Job"
        description={`Delete "${deleteTarget?.name || deleteTarget?.id}"? This cannot be undone.`}
        confirmLabel="Delete"
        variant="danger"
      />
    </div>
  );
}

function FilterBar({
  statusFilter,
  setStatusFilter,
  typeFilter,
  setTypeFilter,
  tagFilter,
  setTagFilter,
  sort,
  setSort,
}: {
  statusFilter: string;
  setStatusFilter: (v: string) => void;
  typeFilter: string;
  setTypeFilter: (v: string) => void;
  tagFilter: string;
  setTagFilter: (v: string) => void;
  sort: SortOption;
  setSort: (v: SortOption) => void;
}) {
  return (
    <div className="bg-bg-panel border border-border rounded-lg p-3 flex items-center gap-3 flex-wrap">
      <Select
        value={statusFilter}
        onChange={(e) => setStatusFilter(e.target.value)}
      >
        {statusOptions.map((opt) => (
          <option key={opt.value} value={opt.value}>{opt.label}</option>
        ))}
      </Select>
      <Select
        value={typeFilter}
        onChange={(e) => setTypeFilter(e.target.value)}
      >
        {typeOptions.map((opt) => (
          <option key={opt.value} value={opt.value}>{opt.label}</option>
        ))}
      </Select>
      <input
        type="text"
        value={tagFilter}
        onChange={(e) => setTagFilter(e.target.value)}
        placeholder="Filter by tag..."
        className="px-3 py-1.5 text-sm bg-bg-input border border-border rounded-md text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-1 focus:ring-ring"
      />
      <div className="flex-1" />
      <Select
        value={sort}
        onChange={(e) => setSort(e.target.value as SortOption)}
      >
        {sortOptions.map((opt) => (
          <option key={opt.value} value={opt.value}>{opt.label}</option>
        ))}
      </Select>
    </div>
  );
}
