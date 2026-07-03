# syntax=docker/dockerfile:1

FROM node:24-alpine AS racingline-build

WORKDIR /workspace/app/racingline

COPY app/racingline/package.json app/racingline/package-lock.json ./
RUN npm ci

COPY app/racingline/ ./

ARG VITE_REARVIEW_API_BASE_URL=""
ARG RACINGLINE_GIT_SHA=container
ENV VITE_REARVIEW_API_BASE_URL=${VITE_REARVIEW_API_BASE_URL}
ENV RACINGLINE_GIT_SHA=${RACINGLINE_GIT_SHA}

RUN npm run build

FROM nginx:1.29-alpine

COPY deploy/nginx/default.conf /etc/nginx/conf.d/default.conf
COPY --from=racingline-build /workspace/app/racingline/dist /usr/share/nginx/html

EXPOSE 80
