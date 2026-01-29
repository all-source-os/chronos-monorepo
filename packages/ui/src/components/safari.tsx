import { SVGProps } from "react";

export interface SafariProps extends SVGProps<SVGSVGElement> {
  url?: string;
  src?: string;
  width?: number;
  height?: number;
}

export function Safari({
  src,
  url,
  width = 1203,
  height = 753,
  ...props
}: SafariProps) {
  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <g clipPath="url(#path0)">
        <path
          d="M0 52H1202V741C1202 747.627 1196.63 753 1190 753H12C5.37258 753 0 747.627 0 741V52Z"
          className="fill-[#E5E5E5] dark:fill-[#404040]"
        />
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M0 12C0 5.37258 5.37258 0 12 0H1190C1196.63 0 1202 5.37258 1202 12V52H0L0 12Z"
          className="fill-[#E5E5E5] dark:fill-[#404040]"
        />
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M1202 51H0V52H1202V51Z"
          className="fill-[#C4C4C4] dark:fill-[#B3B3B3]"
        />
        <circle cx="27" cy="25" r="6" className="fill-[#E95A4F]" />
        <circle cx="47" cy="25" r="6" className="fill-[#F7BD2F]" />
        <circle cx="67" cy="25" r="6" className="fill-[#67C430]" />
        <path
          d="M286 17C286 12.5817 289.582 9 294 9H908C912.418 9 916 12.5817 916 17V35C916 39.4183 912.418 43 908 43H294C289.582 43 286 39.4183 286 35V17Z"
          className="fill-[#F4F4F5] dark:fill-[#525252]"
        />
        <path
          d="M291.5 26C291.5 27.3807 290.381 28.5 289 28.5C287.619 28.5 286.5 27.3807 286.5 26C286.5 24.6193 287.619 23.5 289 23.5C290.381 23.5 291.5 24.6193 291.5 26Z"
          className="fill-[#C7C7C7] dark:fill-[#6B6B6B]"
        />
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M289 29C290.657 29 292 27.6569 292 26C292 24.3431 290.657 23 289 23C287.343 23 286 24.3431 286 26C286 27.6569 287.343 29 289 29ZM289 28C290.105 28 291 27.1046 291 26C291 24.8954 290.105 24 289 24C287.895 24 287 24.8954 287 26C287 27.1046 287.895 28 289 28Z"
          className="fill-[#C7C7C7] dark:fill-[#6B6B6B]"
        />
        <path
          d="M571.647 20.2451C571.136 19.9157 571.139 19.1714 571.652 18.8457L583.766 11.1503C584.335 10.789 585.08 11.2004 585.077 11.8748L585.027 38.1222C585.024 38.7977 584.275 39.2037 583.708 38.8376L571.647 20.2451Z"
          className="fill-[#C7C7C7] dark:fill-[#6B6B6B]"
        />
        <path
          d="M618.647 30.2451C618.136 30.5743 617.139 30.8284 617.652 31.1541L629.766 38.8495C630.335 39.2108 631.08 38.7994 631.077 38.125L631.027 11.8776C631.024 11.2021 630.275 10.7961 629.708 11.1622L618.647 30.2451Z"
          className="fill-[#C7C7C7] dark:fill-[#6B6B6B]"
        />
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M286.5 17C286.5 12.8579 289.858 9.5 294 9.5H908C912.142 9.5 915.5 12.8579 915.5 17V35C915.5 39.1421 912.142 42.5 908 42.5H294C289.858 42.5 286.5 39.1421 286.5 35V17ZM294 10.5C290.41 10.5 287.5 13.4102 287.5 17V35C287.5 38.5899 290.41 41.5 294 41.5H908C911.59 41.5 914.5 38.5899 914.5 35V17C914.5 13.4102 911.59 10.5 908 10.5H294Z"
          className="fill-[#C7C7C7] dark:fill-[#6B6B6B]"
        />
        {url && (
          <text
            x="601"
            y="30"
            textAnchor="middle"
            className="fill-[#A0A0A0] dark:fill-[#8C8C8C]"
            fontSize="12"
          >
            {url}
          </text>
        )}
        {src && (
          <image
            href={src}
            width={width}
            height={height - 52}
            x="0"
            y="52"
            clipPath="url(#roundedBottom)"
            preserveAspectRatio="xMidYMin slice"
          />
        )}
      </g>
      <defs>
        <clipPath id="path0">
          <rect width={width} height={height} fill="white" rx="12" />
        </clipPath>
        <clipPath id="roundedBottom">
          <path d={`M0 52H${width}V${height - 12}C${width} ${height - 6} ${width - 6} ${height} ${width - 12} ${height}H12C6 ${height} 0 ${height - 6} 0 ${height - 12}V52Z`} />
        </clipPath>
      </defs>
    </svg>
  );
}

export default Safari;
