import type { Metadata } from "next";
import { Space_Grotesk, Inter, JetBrains_Mono, Instrument_Serif } from "next/font/google";
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
      <body>
          {children}
          <DevAgentation />
        </body>
    </html>
  );
}
