FROM mcr.microsoft.com/playwright:v1.63.0-noble
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@10.30.3 --activate
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY src ./src
USER pwuser
ENV HOME=/work/home PI_OFFLINE=1
WORKDIR /work
ENTRYPOINT ["node", "--import", "/app/node_modules/tsx/dist/loader.mjs", "/app/src/worker.ts"]
