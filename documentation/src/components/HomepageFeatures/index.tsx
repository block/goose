import type { ReactNode } from "react";
import clsx from "clsx";
import Heading from "@theme/Heading";
import styles from "./styles.module.css";

type FeatureItem = {
  title: string;
  Svg: React.ComponentType<React.ComponentProps<"svg">>;
  description: ReactNode;
};

type FeatureQuote = {
  name: string;
  github: string;
  role: string;
  testimonial: string;
};

const FeatureList: FeatureItem[] = [
  {
    title: "Open Source",
    Svg: require("@site/static/img/lock-unlocked-fill.svg").default,
    description: (
      <>
        Built with transparency and collaboration in mind, goose empowers
        developers to contribute, customize, and innovate freely.
      </>
    ),
  },
  {
    title: "Runs Locally",
    Svg: require("@site/static/img/category-moving.svg").default,
    description: (
      <>
        Goose runs locally to execute tasks efficiently, keeping control in your
        hands.
      </>
    ),
  },
  {
    title: "Extensible",
    Svg: require("@site/static/img/category-ETF.svg").default,
    description: (
      <>
        Customize goose with your preferred LLM and enhance its capabilities by connecting it to any
        external MCP server or API.
      </>
    ),
  },
  {
    title: "Autonomous",
    Svg: require("@site/static/img/pay-in-four.svg").default,
    description: (
      <>
        Goose independently handles complex tasks, from debugging to deployment,
        freeing you to focus on what matters most.
      </>
    ),
  },
];

const FeatureQuotes: FeatureQuote[] = [
  {
    name: "Prem Pillai",
    github: "https://github.com/cloud-on-prem",
    role: "Software Engineer",
    testimonial:
      "With Goose, I feel like I am Maverick. Thanks a ton for creating this. 🙏 I have been having way too much fun with it today.",
  },
  {
    name: "Jarrod Sibbison",
    github: "https://github.com/jsibbison-square",
    role: "Software Engineer",
    testimonial:
      "I wanted to construct some fake data for an API with a large request body and business rules I haven't memorized. So I told Goose which object to update and a test to run that calls the vendor. Got it to use the errors descriptions from the vendor response to keep correcting the request until it was successful. So good!",
  },
  {
    name: "Manik Surtani",
    github: "https://github.com/maniksurtani",
    role: "Head of Open Source",
    testimonial:
      "I asked Goose to write up a few Google Scripts that mimic Clockwise's functionality (particularly, creating blocks on my work calendar based on events in my personal calendar, as well as color-coding calendar entries based on type and importance). Took me under an hour. If you haven't tried Goose yet, I highly encourage you to do so!",
  },
  {
    name: "Andrey Bolduzev",
    github: "https://github.com/andrey-bolduzev",
    role: "Android Engineer",
    testimonial:
      "If anyone was looking for another reason to check it out: I just asked Goose to break a string-array into individual string resources across eleven localizations, and it performed amazingly well and saved me a bunch of time doing it manually or figuring out some way to semi-automate it.",
  },
  {
    name: "Kang Huang",
    github: "https://github.com/kang-square",
    role: "Software Engineer",
    testimonial:
      "Hi team, thank you for much for making Goose, it's so amazing. Our team is working on migrating Dashboard components to React components. I am working with Goose to help the migration.",
  },
  {
    name: "Lily Delalande",
    github: "https://github.com/lily-de",
    role: "Software Engineer",
    testimonial:
      "Wanted to document what I had Goose do -- took about 30 minutes end to end! I created a custom CLI command in the gh CLI library to download in-line comments on PRs about code changes (currently they aren't directly viewable). I don't know Go that well and I definitely didn't know where to start looking in the code base or how to even test the new command was working and Goose did it all for me 😁",
  },
  {
    name: "Rizel Scarlett",
    github: "blackgirlbytes",
    role: "Developer Advocate",
    testimonial:
      "My sister had been asking me for months to help her build a Google Docs extension but I kept putting it off. Today, we hopped on FaceTime and built one in just 30 minutes with Goose!",
  }
  //Cant find Kristens github
  // {
  //   name: "Kristen Anderson",
  //   github: "https://github.com/",
  //   role: "Product Lead",
  //   testimonial:
  //     "I know Goose is made for engineers, but I’ve been pretty excited about this launch. In the last 20 minutes playing with Goose, I drafted a product requirements document for a small change that I'd been putting off, pressure tested a few product ideas and how they might stack up to competitive offerings, and had it run analysis on an experiment from a feature rollout!",
  // },
];

function Feature({ title, Svg, description }: FeatureItem) {
  return (
    <div className={clsx("col col--3")}>
      <div className="text--left padding-horiz--md">
        <Svg className={styles.featureIcon} role="img" />
      </div>
      <div className="text--left padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

function Quote({ name, github, role, testimonial }: FeatureQuote) {
  return (
    <div className="col col--6">
      {/* inline styles in the interest of time */}
      <div
        className="text--left padding-horiz--md padding-bottom--xl"
        style={{
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          alignItems: "left",
        }}
      >
        <div className="avatar">
          <img
            className="avatar__photo"
            src={`https://github.com/${github.split("/").pop()}.png`}
            alt={`${name}'s profile picture`}
          />
          <div className="avatar__intro">
            <div className="avatar__name">{name}</div>
            <small className="avatar__subtitle">{role}</small>
          </div>
        </div>
        <p>{testimonial}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">


          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}

          {/* Testimonials Section */}
          <div style={{ display: "flex", flexDirection: "column", marginTop: "60px" }}>
            <h3 style={{ textAlign: "center", marginBottom: "40px" }}>Loved by engineers</h3>
            <div style={{ display: "flex", flexWrap: "wrap" }}>
              {FeatureQuotes.map((props, idx) => (
                <Quote key={idx} {...props} />
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
