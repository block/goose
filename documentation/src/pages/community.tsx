import type { ReactNode } from "react";
import Link from "@docusaurus/Link";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import Layout from "@theme/Layout";
import Heading from "@theme/Heading";

import styles from "./index.module.css";



function UpcomingEventsSection() {
  return (
    <section className="container margin-vert--lg">
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h1">Upcoming Events</Heading>
            <p>Join us for community events, workshops, and discussions about Goose.</p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12 margin-bottom--lg">
          <div className="card">
            <div className="card__header">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <Heading as="h3">🗓️ No Events Scheduled</Heading>
                <span className="badge badge--secondary">Coming Soon</span>
              </div>
            </div>
            <div className="card__body">
              <p>
                We're currently planning exciting community events! Check back soon for upcoming 
                workshops, AMAs, and community showcases. In the meantime, join our Discord 
                to stay updated and suggest event ideas.
              </p>
              <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
                <Link 
                  className="button button--primary" 
                  href="https://discord.gg/block-opensource"
                >
                  Join Discord for Updates
                </Link>
                <Link 
                  className="button button--outline button--primary" 
                  to="/blog"
                >
                  Read Latest News
                </Link>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center">
            <p style={{ fontStyle: 'italic', color: 'var(--ifm-color-emphasis-600)' }}>
              Want to organize a community event or have ideas for workshops? 
              Reach out to us on <Link href="https://discord.gg/block-opensource">Discord</Link> or 
              create a discussion on <Link href="https://github.com/block/goose/discussions">GitHub</Link>.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

function CommunityAllStarsSection() {
  return (
    <section className="container margin-vert--lg">
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h1">Community All Stars</Heading>
            <p>Celebrating our most active contributors and community champions.</p>
          </div>
        </div>
      </div>
      
      {/* First Group of 5 */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--md">
            <Heading as="h3">Featured Contributors</Heading>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--2 col--offset-1 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/blackgirlbytes.png"
                  alt="Rizel Scarlett"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/blackgirlbytes">
                    Rizel Scarlett
                  </Link>
                </strong>
                <br />
                <small>@blackgirlbytes</small>
                <br />
                <small>Developer Advocate</small>
              </div>
            </div>
          </div>
        </div>
        
        <div className="col col--2 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/zanesq.png"
                  alt="Zane Squires"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/zanesq">
                    Zane Squires
                  </Link>
                </strong>
                <br />
                <small>@zanesq</small>
                <br />
                <small>Senior Software Engineer</small>
              </div>
            </div>
          </div>
        </div>
        
        <div className="col col--2 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/The-Best-Codes.png"
                  alt="The-Best-Codes"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/The-Best-Codes">
                    The-Best-Codes
                  </Link>
                </strong>
                <br />
                <small>@The-Best-Codes</small>
                <br />
                <small>Open Source Developer</small>
              </div>
            </div>
          </div>
        </div>
        
        <div className="col col--2 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/michaelneale.png"
                  alt="Michael Neale"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/michaelneale">
                    Michael Neale
                  </Link>
                </strong>
                <br />
                <small>@michaelneale</small>
                <br />
                <small>Principal Engineer</small>
              </div>
            </div>
          </div>
        </div>
        
        <div className="col col--2 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/patrickReiis.png"
                  alt="Patrick Reis"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/patrickReiis">
                    Patrick Reis
                  </Link>
                </strong>
                <br />
                <small>@patrickReiis</small>
                <br />
                <small>Community Contributor</small>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      {/* Second Group of 5 */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--md">
            <Heading as="h3">Rising Stars</Heading>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--2 col--offset-1 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/angiejones.png"
                  alt="Angie Jones"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/angiejones">
                    Angie Jones
                  </Link>
                </strong>
                <br />
                <small>@angiejones</small>
                <br />
                <small>Senior Developer</small>
              </div>
            </div>
          </div>
        </div>
        
        <div className="col col--2 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/svenstaro.png"
                  alt="Sven-Hendrik Haase"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/svenstaro">
                    Sven-Hendrik Haase
                  </Link>
                </strong>
                <br />
                <small>@svenstaro</small>
                <br />
                <small>Open Source Maintainer</small>
              </div>
            </div>
          </div>
        </div>
        
        <div className="col col--2 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/faces-of-eth.png"
                  alt="faces-of-eth"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/faces-of-eth">
                    faces-of-eth
                  </Link>
                </strong>
                <br />
                <small>@faces-of-eth</small>
                <br />
                <small>Community Developer</small>
              </div>
            </div>
          </div>
        </div>
        
        <div className="col col--2 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/wesbillman.png"
                  alt="Wes Billman"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/wesbillman">
                    Wes Billman
                  </Link>
                </strong>
                <br />
                <small>@wesbillman</small>
                <br />
                <small>Software Engineer</small>
              </div>
            </div>
          </div>
        </div>
        
        <div className="col col--2 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <img
                  className="avatar__photo avatar__photo--lg"
                  src="https://github.com/acheong08.png"
                  alt="Antonio Cheong"
                />
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>
                  <Link href="https://github.com/acheong08">
                    Antonio Cheong
                  </Link>
                </strong>
                <br />
                <small>@acheong08</small>
                <br />
                <small>AI Developer</small>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      {/* Third Group - All Contributors Leaderboard */}
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--md">
            <Heading as="h3">🏆 Contributors Leaderboard</Heading>
            <p style={{ fontSize: '14px', color: 'var(--ifm-color-emphasis-600)' }}>
              All 43 amazing contributors who make Goose possible!
            </p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--6 col--offset-3">
          <div className="card" style={{ padding: '20px' }}>
            <div style={{ 
              display: 'flex', 
              flexDirection: 'column',
              gap: '8px',
              fontSize: '14px'
            }}>
              <div style={{ display: 'flex', alignItems: 'center', padding: '8px', backgroundColor: '#FFD700', borderRadius: '6px', fontWeight: 'bold' }}>
                <span style={{ marginRight: '8px', fontSize: '16px' }}>🥇</span>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>1.</span>
                <Link href="https://github.com/blackgirlbytes" style={{ color: '#000' }}>@blackgirlbytes</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '8px', backgroundColor: '#C0C0C0', borderRadius: '6px', fontWeight: 'bold' }}>
                <span style={{ marginRight: '8px', fontSize: '16px' }}>🥈</span>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>2.</span>
                <Link href="https://github.com/zanesq" style={{ color: '#000' }}>@zanesq</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '8px', backgroundColor: '#CD7F32', borderRadius: '6px', fontWeight: 'bold' }}>
                <span style={{ marginRight: '8px', fontSize: '16px' }}>🥉</span>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>3.</span>
                <Link href="https://github.com/michaelneale" style={{ color: '#000' }}>@michaelneale</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>4.</span>
                <Link href="https://github.com/angiejones">@angiejones</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>5.</span>
                <Link href="https://github.com/Kvadratni">@Kvadratni</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>6.</span>
                <Link href="https://github.com/lifeizhou-ap">@lifeizhou-ap</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>7.</span>
                <Link href="https://github.com/dianed-square">@dianed-square</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>8.</span>
                <Link href="https://github.com/yingjiehe-xyz">@yingjiehe-xyz</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>9.</span>
                <Link href="https://github.com/salman1993">@salman1993</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>10.</span>
                <Link href="https://github.com/ahau-square">@ahau-square</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>11.</span>
                <Link href="https://github.com/iandouglas">@iandouglas</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>12.</span>
                <Link href="https://github.com/emma-squared">@emma-squared</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>13.</span>
                <Link href="https://github.com/dbraduan">@dbraduan</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>14.</span>
                <Link href="https://github.com/lily-de">@lily-de</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>15.</span>
                <Link href="https://github.com/alexhancock">@alexhancock</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>16.</span>
                <Link href="https://github.com/EbonyLouis">@EbonyLouis</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>17.</span>
                <Link href="https://github.com/wendytang">@wendytang</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>18.</span>
                <Link href="https://github.com/The-Best-Codes">@The-Best-Codes</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>19.</span>
                <Link href="https://github.com/opdich">@opdich</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>20.</span>
                <Link href="https://github.com/agiuliano-square">@agiuliano-square</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>21.</span>
                <Link href="https://github.com/patrickReiis">@patrickReiis</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>22.</span>
                <Link href="https://github.com/kalvinnchau">@kalvinnchau</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>23.</span>
                <Link href="https://github.com/acekyd">@acekyd</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>24.</span>
                <Link href="https://github.com/nahiyankhan">@nahiyankhan</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>25.</span>
                <Link href="https://github.com/taniashiba">@taniashiba</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>26.</span>
                <Link href="https://github.com/JohnMAustin78">@JohnMAustin78</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>27.</span>
                <Link href="https://github.com/sheagcraig">@sheagcraig</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>28.</span>
                <Link href="https://github.com/alicehau">@alicehau</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>29.</span>
                <Link href="https://github.com/bwalding">@bwalding</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>30.</span>
                <Link href="https://github.com/jamadeo">@jamadeo</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>31.</span>
                <Link href="https://github.com/rockwotj">@rockwotj</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>32.</span>
                <Link href="https://github.com/danielzayas">@danielzayas</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>33.</span>
                <Link href="https://github.com/svenstaro">@svenstaro</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>34.</span>
                <Link href="https://github.com/adaug">@adaug</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>35.</span>
                <Link href="https://github.com/loganmoseley">@loganmoseley</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>36.</span>
                <Link href="https://github.com/tiborvass">@tiborvass</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>37.</span>
                <Link href="https://github.com/xuv">@xuv</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>38.</span>
                <Link href="https://github.com/anilmuppalla">@anilmuppalla</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>39.</span>
                <Link href="https://github.com/spencrmartin">@spencrmartin</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>40.</span>
                <Link href="https://github.com/gknoblauch">@gknoblauch</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>41.</span>
                <Link href="https://github.com/acheong08">@acheong08</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>42.</span>
                <Link href="https://github.com/faces-of-eth">@faces-of-eth</Link>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', padding: '6px', backgroundColor: '#f8f9fa', borderRadius: '4px' }}>
                <span style={{ marginRight: '8px', minWidth: '20px' }}>43.</span>
                <Link href="https://github.com/wesbillman">@wesbillman</Link>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-top--lg">
            <p style={{ fontStyle: 'italic', color: 'var(--ifm-color-emphasis-600)', fontSize: '14px' }}>
              Thank you to all our amazing contributors! 🙏
            </p>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center margin-bottom--lg">
            <Heading as="h2">Want to be featured?</Heading>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--4 col--offset-4 margin-bottom--lg">
          <div className="card">
            <div className="card__header text--center">
              <div className="avatar avatar--vertical">
                <div style={{ 
                  width: '64px', 
                  height: '64px', 
                  borderRadius: '50%', 
                  backgroundColor: '#f0f0f0', 
                  display: 'flex', 
                  alignItems: 'center', 
                  justifyContent: 'center',
                  fontSize: '20px',
                  color: '#666'
                }}>
                  ?
                </div>
              </div>
            </div>
            <div className="card__body text--center">
              <div className="margin-bottom--sm">
                <strong>Your Name Here</strong>
                <br />
                <small>Future Community Star</small>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <div className="row">
        <div className="col col--12">
          <div className="text--center">
            <p style={{ fontStyle: 'italic', color: 'var(--ifm-color-emphasis-600)' }}>
              Want to be featured as a Community All Star? Start contributing on{' '}
              <Link href="https://github.com/block/goose">GitHub</Link>, help others on{' '}
              <Link href="https://discord.gg/block-opensource">Discord</Link>, or share your 
              Goose projects with the community!
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}





export default function Community(): ReactNode {
  const { siteConfig } = useDocusaurusContext();
  return (
    <Layout 
      title="Community" 
      description="Join the Goose community - connect with developers, contribute to the project, and help shape the future of AI-powered development tools."
    >
      <main>
        <UpcomingEventsSection />
        <CommunityAllStarsSection />
      </main>
    </Layout>
  );
}