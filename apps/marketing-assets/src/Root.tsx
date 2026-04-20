import { Composition, Still } from "remotion";
import { HeroScreenshot } from "./compositions/HeroScreenshot";
import { DemoVideo } from "./compositions/DemoVideo";
import { GifEventExplorer } from "./compositions/GifEventExplorer";
import { GifLiveStream } from "./compositions/GifLiveStream";
import { GifApiKeyCreation } from "./compositions/GifApiKeyCreation";
import { GifOnboarding } from "./compositions/GifOnboarding";
import { blogHeaders, makeBlogHeader } from "./compositions/BlogHeaders";

// Resolutions sized for retina displays. Feature videos display at
// ~600-900px wide in the browser, so 1600x1000 gives crisp 2x on
// most screens. Hero is 2540x1520 (2x of 1270x760 display size).
// Blog headers are 1200x630 (OG image standard).

export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Still id="hero-screenshot" component={HeroScreenshot} width={2540} height={1520} />

      <Composition id="demo-video" component={DemoVideo} durationInFrames={1800} fps={30} width={1920} height={1080} />

      <Composition id="gif-event-explorer" component={GifEventExplorer} durationInFrames={150} fps={30} width={1600} height={1000} />
      <Composition id="gif-live-stream" component={GifLiveStream} durationInFrames={150} fps={30} width={1600} height={1000} />
      <Composition id="gif-api-key" component={GifApiKeyCreation} durationInFrames={120} fps={30} width={1600} height={1000} />
      <Composition id="gif-onboarding" component={GifOnboarding} durationInFrames={150} fps={30} width={1600} height={1000} />

      {/* Blog header images — 1200x630 (OG standard) */}
      {Object.keys(blogHeaders).map((slug) => (
        <Still key={slug} id={`blog-${slug}`} component={makeBlogHeader(slug)} width={1200} height={630} />
      ))}
    </>
  );
};
