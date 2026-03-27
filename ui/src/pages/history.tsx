import { useState, useEffect, useCallback } from "react";
import { Link, useSearchParams } from "react-router-dom";
import {
  Copy,
  ClockCounterClockwise,
  Clock,
  HandPointing,
} from "@phosphor-icons/react";
import { useApi } from "../hooks/use-api";
import { useSSE } from "../hooks/use-sse";
import { historyApi, jobsApi } from "../api";
import type { RunRecord, JobListEntry } from "../types";
import { StatusBadge } from "../components/shared/status-badge";
import { Select } from "../components/ui/select";
import { Button } from "../components/ui/button";
import { EmptyState } from "../components/ui/empty-state";
import { TableSkeleton } from "../components/ui/skeleton";
import { formatDurationBetween, formatDateTime } from "../lib/format";
import { cn } from "../lib/cn";

const LIMIT = 50;

const STATUS_OPTIONS = [
  { value: "", label: "All" },
  { value: "success", label: "Success" },
  { value: "failed", label: "Failed" },
  { value: "timeout", label: "Timeout" },
  { value: "skipped_overlap", label: "Skipped" },
  { value: "internal_error", label: "Error" },
];

const TRIGGER_OPTIONS = [
  { value: "", label: "All" },
  { value: "scheduled", label: "Scheduled" },
  { value: "manual", label: "Manual" },
];

const SORT_OPTIONS = [
  { value: "newest", label: "Newest First" },
  { value: "oldest", label: "Oldest First" },
  { value: "duration", label: "Duration (Longest)" },
];

function CopyRunId({ runId }: { runId: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      navigator.clipboard.writeText(runId);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    },
    [runId],
  );

  return (
    <button
      onClick={handleCopy}
      className="ml-1.5 opacity-0 group-hover/row:opacity-100 transition-opacity text-text-muted hover:text-text-primary"
      title="Copy Run ID"
    >
      {copied ? (
        <span className="text-xs text-green-500">Copied</span>
      ) : (
        <Copy size={14} />
      )}
    </button>
  );
}

function TriggerBadge({ trigger }: { trigger: string }) {
  const isScheduled = trigger === "scheduled";
  return (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs border border-border rounded-md text-text-secondary">
      {isScheduled ? <Clock size={12} /> : <HandPointing size={12} />}
      {isScheduled ? "Scheduled" : "Manual"}
    </span>
  );
}

export default function HistoryPage() {
  const [searchParams, setSearchParams] = useSearchParams();

  const [status, setStatus] = useState(searchParams.get("status") ?? "");
  const [jobId, setJobId] = useState(searchParams.get("job") ?? "");
  const [trigger, setTrigger] = useState(searchParams.get("trigger") ?? "");
  const [dateFrom, setDateFrom] = useState(searchParams.get("from") ?? "");
  const [dateTo, setDateTo] = useState(searchParams.get("to") ?? "");
  const [sort, setSort] = useState(searchParams.get("sort") ?? "newest");
  const [offset, setOffset] = useState(
    parseInt(searchParams.get("offset") ?? "0", 10),
  );

  // Sync filters to URL
  useEffect(() => {
    const params: Record<string, string> = {};
    if (status) params.status = status;
    if (jobId) params.job = jobId;
    if (trigger) params.trigger = trigger;
    if (dateFrom) params.from = dateFrom;
    if (dateTo) params.to = dateTo;
    if (sort && sort !== "newest") params.sort = sort;
    if (offset > 0) params.offset = String(offset);
    setSearchParams(params, { replace: true });
  }, [status, jobId, trigger, dateFrom, dateTo, sort, offset, setSearchParams]);

  const apiParams = {
    status: status || undefined,
    job_id: jobId || undefined,
    trigger: trigger || undefined,
    from: dateFrom || undefined,
    to: dateTo || undefined,
    sort: sort || undefined,
    limit: LIMIT,
    offset,
  };

  const {
    data: historyData,
    loading,
    refetch,
  } = useApi(() => historyApi.list(apiParams), [
    status,
    jobId,
    trigger,
    dateFrom,
    dateTo,
    sort,
    offset,
  ]);

  const { data: jobsData } = useApi(() => jobsApi.list({}), []);

  useSSE((event) => { if (event === 'run-completed') refetch() });

  const records: RunRecord[] = historyData?.records ?? [];
  const total = historyData?.total ?? 0;
  const jobs: JobListEntry[] = jobsData ?? [];

  const rangeStart = total === 0 ? 0 : offset + 1;
  const rangeEnd = Math.min(offset + LIMIT, total);
  const hasPrev = offset > 0;
  const hasNext = offset + LIMIT < total;

  const resetFilters = () => {
    setStatus("");
    setJobId("");
    setTrigger("");
    setDateFrom("");
    setDateTo("");
    setSort("newest");
    setOffset(0);
  };

  const handleFilterChange = (setter: (v: string) => void) => (e: React.ChangeEvent<HTMLSelectElement>) => {
    setter(e.target.value);
    setOffset(0);
  };

  return (
    <div className="flex flex-col gap-4 h-full">
      <div className="flex items-center gap-3">
        <ClockCounterClockwise size={24} className="text-text-primary" />
        <h1 className="text-xl font-semibold text-text-primary">
          Run History
        </h1>
      </div>

      {/* Filter bar */}
      <div className="bg-bg-panel border border-border rounded-lg p-3 flex flex-wrap items-end gap-3">
        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">Status</label>
          <Select
            value={status}
            onChange={handleFilterChange(setStatus)}
            options={STATUS_OPTIONS}
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">Job</label>
          <Select
            value={jobId}
            onChange={handleFilterChange(setJobId)}
            options={[
              { value: "", label: "All Jobs" },
              ...jobs.map((j) => ({
                value: j.id,
                label: j.name ?? j.id,
              })),
            ]}
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">Trigger</label>
          <Select
            value={trigger}
            onChange={handleFilterChange(setTrigger)}
            options={TRIGGER_OPTIONS}
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">From</label>
          <input
            type="date"
            value={dateFrom}
            onChange={(e) => {
              setDateFrom(e.target.value);
              setOffset(0);
            }}
            className="h-9 px-2 text-sm bg-bg-input border border-border rounded-md text-text-primary"
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">To</label>
          <input
            type="date"
            value={dateTo}
            onChange={(e) => {
              setDateTo(e.target.value);
              setOffset(0);
            }}
            className="h-9 px-2 text-sm bg-bg-input border border-border rounded-md text-text-primary"
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">Sort</label>
          <Select
            value={sort}
            onChange={handleFilterChange(setSort)}
            options={SORT_OPTIONS}
          />
        </div>

        <Button variant="ghost" onClick={resetFilters}>
          Reset
        </Button>
      </div>

      {/* Table */}
      <div className="bg-bg-panel border border-border rounded-lg flex-1 overflow-auto">
        {loading ? (
          <TableSkeleton rows={10} cols={9} />
        ) : records.length === 0 ? (
          <EmptyState
            icon={<ClockCounterClockwise size={48} />}
            title="No runs found"
            description="Adjust your filters or wait for scheduled jobs to execute."
          />
        ) : (
          <table className="w-full text-sm">
            <thead className="sticky top-0 z-10 bg-bg-panel border-b border-border">
              <tr className="text-left text-text-muted text-xs uppercase tracking-wider">
                <th className="px-4 py-3 font-medium">Run ID</th>
                <th className="px-4 py-3 font-medium">Job Name</th>
                <th className="px-4 py-3 font-medium">Trigger</th>
                <th className="px-4 py-3 font-medium">Scheduled For</th>
                <th className="px-4 py-3 font-medium">Started</th>
                <th className="px-4 py-3 font-medium">Finished</th>
                <th className="px-4 py-3 font-medium">Duration</th>
                <th className="px-4 py-3 font-medium">Status</th>
                <th className="px-4 py-3 font-medium text-right">
                  Exit Code
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {records.map((record) => (
                <tr
                  key={record.run_id}
                  className="group/row hover:bg-bg-hover transition-colors cursor-pointer"
                >
                  <td className="px-4 py-3">
                    <Link
                      to={`/runs/${record.run_id}`}
                      className="inline-flex items-center"
                    >
                      <span className="font-mono text-xs text-text-code">
                        {record.run_id.slice(0, 8)}
                      </span>
                      <CopyRunId runId={record.run_id} />
                    </Link>
                  </td>
                  <td className="px-4 py-3">
                    <Link
                      to={`/jobs/${record.job_id}`}
                      className="hover:text-accent transition-colors"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {record.job_name}
                    </Link>
                  </td>
                  <td className="px-4 py-3">
                    <TriggerBadge trigger={record.trigger} />
                  </td>
                  <td className="px-4 py-3 text-text-secondary">
                    {record.scheduled_for
                      ? formatDateTime(record.scheduled_for)
                      : "-"}
                  </td>
                  <td className="px-4 py-3 text-text-secondary">
                    {record.started_at
                      ? formatDateTime(record.started_at)
                      : "-"}
                  </td>
                  <td className="px-4 py-3 text-text-secondary">
                    {record.finished_at
                      ? formatDateTime(record.finished_at)
                      : "-"}
                  </td>
                  <td className="px-4 py-3 text-text-secondary">
                    {record.started_at && record.finished_at
                      ? formatDurationBetween(
                          record.started_at,
                          record.finished_at,
                        )
                      : "-"}
                  </td>
                  <td className="px-4 py-3">
                    <StatusBadge status={record.status} />
                  </td>
                  <td
                    className={cn(
                      "px-4 py-3 text-right font-mono text-xs",
                      record.exit_code === 0
                        ? "text-text-muted"
                        : record.exit_code != null
                          ? "text-red-400"
                          : "text-text-muted",
                    )}
                  >
                    {record.exit_code != null ? record.exit_code : "-"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Pagination */}
      {total > 0 && (
        <div className="flex items-center justify-between px-1">
          <span className="text-sm text-text-muted">
            Showing {rangeStart}-{rangeEnd} of {total}
          </span>
          <div className="flex items-center gap-2">
            <Button
              variant="secondary"
              disabled={!hasPrev}
              onClick={() => setOffset(Math.max(0, offset - LIMIT))}
            >
              Previous
            </Button>
            <Button
              variant="secondary"
              disabled={!hasNext}
              onClick={() => setOffset(offset + LIMIT)}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
