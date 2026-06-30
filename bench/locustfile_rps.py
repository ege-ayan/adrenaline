"""Locust load test with a fixed target RPS via constant_pacing.

Set TARGET_RPS and run with --users (must match LOCUST_USERS for pacing math):
  TARGET_RPS=6000 LOCUST_USERS=200 locust -f locustfile_rps.py --users 200 ...
"""

import os

from locust import FastHttpUser, constant_pacing, task

TARGET_RPS = int(os.environ["TARGET_RPS"])
USERS = int(os.environ.get("LOCUST_USERS", "200"))


class BenchUser(FastHttpUser):
    # Total RPS ≈ users / pacing_seconds
    wait_time = constant_pacing(USERS / TARGET_RPS)

    @task
    def index(self):
        self.client.get("/")
