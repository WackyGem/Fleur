export type RacinglineBuildMetadata = {
  appName: string
  appVersion: string
  gitSha: string
  buildTime: string
}

declare const __RACINGLINE_BUILD_METADATA__: RacinglineBuildMetadata

declare global {
  interface Window {
    __RACINGLINE_BUILD_METADATA__?: RacinglineBuildMetadata
  }
}

export const racinglineBuildMetadata = __RACINGLINE_BUILD_METADATA__
