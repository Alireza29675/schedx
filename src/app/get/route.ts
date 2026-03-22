import { NextResponse } from "next/server";

export function GET() {
  return NextResponse.redirect(
    "https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh",
    302
  );
}
