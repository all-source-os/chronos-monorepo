import { ImageResponse } from "next/og";
import type { NextRequest } from "next/server";
import { siteConfig } from "@/lib/config";

export const runtime = "edge";

export async function GET(req: NextRequest) {
  const { searchParams } = req.nextUrl;
  const postTitle = searchParams.get("title") || siteConfig.description;
  const font = fetch(new URL("../../assets/fonts/Inter-SemiBold.ttf", import.meta.url)).then(
    (res) => res.arrayBuffer()
  );
  const fontData = await font;

  return new ImageResponse(
    <div
      style={{
        height: "100%",
        width: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: "#063A6C",
        backgroundImage:
          "radial-gradient(circle at 25% 15%, rgba(129,212,250,.28) 0%, transparent 42%), radial-gradient(circle at 80% 0%, rgba(41,182,246,.22) 0%, transparent 48%)",
        color: "#F7FBFF",
        fontSize: 32,
        fontWeight: 600,
      }}
    >
      <div
        style={{
          position: "relative",
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          alignItems: "center",
          top: "125px",
        }}
      >
        {/* biome-ignore lint/performance/noImgElement: ImageResponse requires a native image element. */}
        <img src={`${siteConfig.url}/logo.png`} alt="AllSource logo" width={72} height={72} />

        <div
          style={{
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            fontSize: "64px",
            fontWeight: "600",
            marginTop: "24px",
            textAlign: "center",
            width: "80%",
            letterSpacing: "-0.05em",
          }}
        >
          {postTitle}
        </div>
        <div
          style={{
            display: "flex",
            fontSize: "16px",
            fontWeight: "500",
            marginTop: "16px",
            color: "#D4E5F7",
          }}
        >
          {siteConfig.name}
        </div>
      </div>

      {/* biome-ignore lint/performance/noImgElement: ImageResponse requires a native image element. */}
      <img
        src={`${siteConfig.url}/dashboard.png`}
        alt="Dashboard preview"
        width={900}
        style={{
          position: "relative",
          bottom: -160,
          border: "4px solid #81D4FA",
          background: "#F7FBFF",
          borderRadius: 20,
        }}
      />
    </div>,
    {
      width: 1200,
      height: 630,
      fonts: [
        {
          name: "Inter",
          data: fontData,
          style: "normal",
        },
      ],
    }
  );
}
