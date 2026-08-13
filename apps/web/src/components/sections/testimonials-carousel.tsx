import {
  Carousel,
  CarouselContent,
  CarouselItem,
  CarouselNext,
  CarouselPrevious,
  Section,
} from "@allsource/ui";
import Image from "next/image";
import { MdOutlineFormatQuote } from "react-icons/md";
import { FadeIn } from "@/components/ui/fade-in";

const companies = [
  "Google",
  "Microsoft",
  "Amazon",
  "Netflix",
  "YouTube",
  "Instagram",
  "Uber",
  "Spotify",
];

export default function Component() {
  return (
    <Section title="Testimonial Highlight" subtitle="What our customers are saying">
      <Carousel>
        <div className="max-w-2xl mx-auto relative">
          <CarouselContent>
            {Array.from({ length: 7 }).map((_, index) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: Testimonial carousel uses index as key since items are identical
              <CarouselItem key={`testimonial-${index}`}>
                <div className="p-2 pb-5">
                  <div className="text-center">
                    <MdOutlineFormatQuote className="text-4xl text-themeDarkGray my-4 mx-auto" />
                    <FadeIn delay={0.25} inView>
                      <h4 className="text-1xl font-semibold max-w-lg mx-auto px-10">
                        There is a lot of exciting stuff going on in the stars above us that make
                        astronomy so much fun. The truth is the universe is a constantly changing,
                        moving, some would say "living" thing because you just never know what you
                        are going to see on any given night of stargazing.
                      </h4>
                    </FadeIn>
                    <FadeIn delay={0.25 * 2} inView>
                      <div className="mt-8">
                        <Image
                          width={0}
                          height={40}
                          src={`https://cdn.magicui.design/companies/${
                            companies[index % companies.length]
                          }.svg`}
                          alt={`${companies[index % companies.length]} Logo`}
                          className="mx-auto w-auto h-[40px] grayscale opacity-30"
                        />
                      </div>
                    </FadeIn>
                    <div className="">
                      <FadeIn delay={0.25 * 3} inView>
                        <h4 className="text-1xl font-semibold my-2">Leslie Alexander</h4>
                      </FadeIn>
                    </div>
                    <FadeIn delay={0.25 * 4} inView>
                      <div className=" mb-3">
                        <span className="text-sm text-themeDarkGray">UI Designer</span>
                      </div>
                    </FadeIn>
                  </div>
                </div>
              </CarouselItem>
            ))}
          </CarouselContent>
          <div className="pointer-events-none absolute inset-y-0 left-0 h-full w-2/12 bg-gradient-to-r from-background" />
          <div className="pointer-events-none absolute inset-y-0 right-0 h-full  w-2/12 bg-gradient-to-l from-background" />
        </div>
        <div className="md:block hidden absolute bottom-0 left-1/2 -translate-x-1/2">
          <CarouselPrevious />
          <CarouselNext />
        </div>
      </Carousel>
    </Section>
  );
}
