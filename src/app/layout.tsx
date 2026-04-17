import type { Metadata } from "next";
import { Space_Grotesk, Inter, JetBrains_Mono, Instrument_Serif } from "next/font/google";
import Script from "next/script";
import "./globals.css";
import { DevAgentation } from "./components/DevAgentation";

const spaceGrotesk = Space_Grotesk({
  variable: "--font-display",
  subsets: ["latin"],
  weight: ["400", "500", "700"],
});

const inter = Inter({
  variable: "--font-body",
  subsets: ["latin"],
  weight: ["400", "500"],
});

const jetbrainsMono = JetBrains_Mono({
  variable: "--font-mono",
  subsets: ["latin"],
  weight: ["400", "700"],
});

const instrumentSerif = Instrument_Serif({
  variable: "--font-serif",
  subsets: ["latin"],
  weight: ["400"],
  style: ["normal", "italic"],
});

export const metadata: Metadata = {
  title: "schedx — Scheduler CLI for recurring jobs, agent prompts, and webhooks",
  description:
    "One tool to schedule everything. Cron jobs, one-shot tasks, webhook calls, and AI agent prompts. Local-first, file-based, built for humans and agents.",
  openGraph: {
    title: "schedx",
    description: "Scheduler CLI for recurring jobs, agent prompts, and webhooks",
    url: "https://schedx.run",
    siteName: "schedx",
    type: "website",
    images: [
      {
        url: "https://schedx.run/og.png",
        width: 1200,
        height: 630,
        alt: "schedx — Scheduling tool. Built for agents and humans.",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "schedx",
    description: "Scheduler CLI for recurring jobs, agent prompts, and webhooks",
    images: ["https://schedx.run/og.png"],
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${spaceGrotesk.variable} ${inter.variable} ${jetbrainsMono.variable} ${instrumentSerif.variable}`}
    >
      <head>
        <Script
          src="https://cloud.umami.is/script.js"
          data-website-id="74342574-d190-4c8a-924a-27ea3f425101"
          strategy="afterInteractive"
        />
      </head>
      <body>
          {children}
          <DevAgentation />
        </body>
    </html>
  );
}
