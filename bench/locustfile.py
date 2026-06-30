from locust import FastHttpUser, task, constant


class BenchUser(FastHttpUser):
    wait_time = constant(0)

    @task
    def index(self):
        self.client.get("/")
