FROM python:3.12-slim

WORKDIR /app

COPY trebuchet_simulator.py .

RUN pip install --no-cache-dir click

ENV SIM_HOST=backend
ENV SIM_PORT=9001
ENV SIM_INTERVAL=60
ENV SIM_MODE=random

ENTRYPOINT ["python", "trebuchet_simulator.py"]
CMD ["--host", "backend", "--port", "9001", "--interval", "60", "--mode", "random"]
