FROM python:3.13-alpine@sha256:1e1f6334529d1fb26a2ef619d7cffb30a7c7d7cc1cc7338ebee283a58d1aeaca
# Controls need only Python's standard library; omit installer/vendor packages.
RUN python -m pip uninstall --yes pip && python -c "import ensurepip,shutil; shutil.rmtree(ensurepip.__path__[0])"
COPY deployments/lab/clients/ /lab/
USER 65532:65532
ENV PYTHONDONTWRITEBYTECODE=1
ENTRYPOINT []
CMD ["python", "/lab/probe.py"]
