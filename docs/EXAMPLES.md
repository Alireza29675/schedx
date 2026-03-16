# Examples

Real-world recipes for `schedx` covering commands, webhooks, and AI agent prompts.

## DevOps & Infrastructure

```bash
# Database backup every night at 2am
schedx add "0 2 * * *" --run "pg_dump mydb > /backups/mydb-$(date +%F).sql" \
  --name "nightly-backup" --tag ops

# Health-check ping every 5 minutes
schedx add "every 5m" --run "curl -fsS https://myapp.com/health || echo UNHEALTHY" \
  --name "health-ping"

# Clear temp files older than 7 days, daily at midnight
schedx add "0 0 * * *" --run "find /tmp -name '*.tmp' -mtime +7 -delete" \
  --name "tmp-cleanup"

# SSL certificate expiry check every Monday at 9am
schedx add "0 9 * * 1" \
  --run "echo | openssl s_client -connect myapp.com:443 2>/dev/null | openssl x509 -noout -enddate" \
  --name "ssl-check" --tag security
```

## Webhooks & Notifications

```bash
# Post a daily standup reminder to Slack at 9:30am on weekdays
schedx add "30 9 * * 1-5" \
  --webhook https://hooks.slack.com/services/T00/B00/xxx \
  --method POST \
  --header "Content-Type: application/json" \
  --body '{"text":"Time for standup!"}' \
  --name "standup-reminder"

# Ping a heartbeat monitor every minute
schedx add "every 1m" \
  --webhook https://hc-ping.com/your-uuid \
  --name "heartbeat"

# Trigger a CI build every 6 hours
schedx add "every 6h" \
  --webhook https://api.github.com/repos/owner/repo/dispatches \
  --method POST \
  --header "Authorization: Bearer ghp_xxx" \
  --header "Content-Type: application/json" \
  --body '{"event_type":"scheduled-build"}' \
  --name "ci-trigger" --tag ci
```

## AI Agent Workflows

```bash
# Daily code review summary at 5pm on weekdays
schedx add "0 17 * * 1-5" \
  --prompt "Summarize today's open pull requests and flag any that have been waiting more than 48 hours" \
  --name "pr-review" --tag ai

# Weekly project health report every Friday at 4pm
schedx add "0 16 * * 5" \
  --prompt "Generate a weekly project health report covering build status, test coverage trends, and open issues" \
  --name "weekly-report" --tag ai

# Morning briefing at 8am on weekdays
schedx add "0 8 * * 1-5" \
  --prompt "What are the top 3 priorities I should focus on today based on my open tasks and calendar?" \
  --name "morning-brief" --tag ai
```

## One-Shot & Timed Tasks

```bash
# Remind me in 30 minutes
schedx add "in 30m" \
  --run "osascript -e 'display notification \"Break time!\" with title \"schedx\"'" \
  --name "break-reminder"

# Run a migration at a specific time
schedx add "2026-04-01T03:00:00Z" \
  --run "cargo run --release -- migrate" \
  --name "db-migration" --timeout 600

# Deploy to staging in 2 hours
schedx add "in 2h" --run "./deploy.sh staging" \
  --name "staging-deploy" --tag deploy
```

## Monitoring & Data Pipelines

```bash
# Check disk usage every 30 minutes, alert if above 90%
schedx add "every 30m" \
  --run "df -h / | awk 'NR==2 {if(\$5+0 > 90) print \"DISK CRITICAL: \" \$5}'" \
  --name "disk-watch" --tag monitoring

# Fetch exchange rates daily at 6am
schedx add "0 6 * * *" \
  --webhook https://api.exchangerate.host/latest?base=USD \
  --name "fx-rates" --tag data

# Rotate application logs weekly on Sunday at 3am
schedx add "0 3 * * 0" --run "logrotate /etc/logrotate.d/myapp" \
  --name "log-rotate" --tag ops
```

## Job Lifecycle Management

```bash
# Create a job
schedx add "every 1h" --run "echo heartbeat" --name "demo"

# List all jobs
schedx list

# Pause and resume
schedx pause <job-id>
schedx resume <job-id>

# Skip the next 3 runs
schedx skip <job-id> --times 3

# View run history and logs
schedx history <job-id>
schedx logs <job-id>

# Remove the job
schedx rm <job-id>
```
