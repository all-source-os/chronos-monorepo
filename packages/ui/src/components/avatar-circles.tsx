"use client";

import Image from "next/image";
import { cn } from "../lib/utils";

interface Avatar {
  imageUrl: string;
  profileUrl: string;
}

interface AvatarCirclesProps {
  className?: string;
  numPeople?: number;
  avatarUrls: Avatar[];
}

export function AvatarCircles({ numPeople, className, avatarUrls }: AvatarCirclesProps) {
  return (
    <div className={cn("z-10 flex -space-x-4 rtl:space-x-reverse", className)}>
      {avatarUrls.map((avatar) => (
        <a
          key={avatar.profileUrl}
          href={avatar.profileUrl}
          target="_blank"
          rel="noopener noreferrer"
        >
          <Image
            className="h-10 w-10 rounded-full border-2 border-white dark:border-gray-800"
            src={avatar.imageUrl}
            width={40}
            height={40}
            alt="Avatar"
          />
        </a>
      ))}
      {numPeople && (
        <span className="flex h-10 w-10 items-center justify-center rounded-full border-2 border-white bg-black text-center text-xs font-medium text-white dark:border-gray-800 dark:bg-white dark:text-black">
          +{numPeople}
        </span>
      )}
    </div>
  );
}

export default AvatarCircles;
