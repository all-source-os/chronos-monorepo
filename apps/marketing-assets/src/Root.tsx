import { Composition, Still } from "remotion";

// Compositions — add as they're built
// import { HeroScreenshot } from "./compositions/HeroScreenshot";
// import { DemoVideo } from "./compositions/DemoVideo";
// import { GifEventExplorer } from "./compositions/GifEventExplorer";
// import { GifLiveStream } from "./compositions/GifLiveStream";
// import { GifApiKeyCreation } from "./compositions/GifApiKeyCreation";
// import { GifOnboarding } from "./compositions/GifOnboarding";

export const RemotionRoot: React.FC = () => {
  return (
    <>
      {/* Uncomment as each composition is built:

      <Still
        id="hero-screenshot"
        component={HeroScreenshot}
        width={1270}
        height={760}
      />

      <Composition
        id="demo-video"
        component={DemoVideo}
        durationInFrames={1800}
        fps={30}
        width={1920}
        height={1080}
      />

      <Composition
        id="gif-event-explorer"
        component={GifEventExplorer}
        durationInFrames={150}
        fps={30}
        width={800}
        height={500}
      />

      <Composition
        id="gif-live-stream"
        component={GifLiveStream}
        durationInFrames={150}
        fps={30}
        width={800}
        height={500}
      />

      <Composition
        id="gif-api-key"
        component={GifApiKeyCreation}
        durationInFrames={120}
        fps={30}
        width={800}
        height={500}
      />

      <Composition
        id="gif-onboarding"
        component={GifOnboarding}
        durationInFrames={150}
        fps={30}
        width={800}
        height={500}
      />

      */}
    </>
  );
};
