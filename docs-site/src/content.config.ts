import { defineCollection } from "astro:content";
import { z } from "astro/zod";
import { docsLoader, i18nLoader } from "@astrojs/starlight/loaders";
import { docsSchema, i18nSchema } from "@astrojs/starlight/schema";

const landingSchema = z
  .object({
    "landing.title": z.string(),
    "landing.lead": z.string(),
    "landing.release": z.string(),
    "landing.release.install": z.string(),
    "landing.release.packages": z.string(),
    "landing.action.firstScene": z.string(),
    "landing.action.sourceFormat": z.string(),
    "landing.scene.kicker": z.string(),
    "landing.scene.title": z.string(),
    "landing.scene.opening": z.string(),
    "landing.scene.technician": z.string(),
    "landing.scene.again": z.string(),
    "landing.scene.recording": z.string(),
    "landing.scene.directions": z.string(),
    "landing.scene.tape": z.string(),
    "landing.scene.power": z.string(),
    "landing.scene.powerResult": z.string(),
    "landing.scene.plan": z.string(),
    "landing.scene.planResult": z.string(),
    "landing.scene.choose": z.string(),
    "landing.path.kicker": z.string(),
    "landing.path.title": z.string(),
    "landing.write.title": z.string(),
    "landing.write.body": z.string(),
    "landing.write.link": z.string(),
    "landing.check.title": z.string(),
    "landing.check.body": z.string(),
    "landing.check.link": z.string(),
    "landing.integrate.title": z.string(),
    "landing.integrate.body": z.string(),
    "landing.integrate.link": z.string(),
    "landing.localisation.title": z.string(),
    "landing.localisation.body1": z.string(),
    "landing.localisation.body2": z.string(),
    "landing.localisation.link": z.string(),
    "landing.fit.title": z.string(),
    "landing.fit.body1": z.string(),
    "landing.fit.body2": z.string(),
    "landing.fit.body3": z.string(),
    "landing.fit.link": z.string(),
    "landing.navigation.label": z.string(),
    "landing.navigation.manual": z.string(),
    "landing.navigation.reference": z.string(),
    "landing.navigation.migration": z.string(),
    "landing.navigation.packages": z.string(),
    "landing.navigation.source": z.string(),
    "landing.licence": z.string(),
  })
  .partial();

export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema(),
  }),
  i18n: defineCollection({
    loader: i18nLoader(),
    schema: i18nSchema({ extend: landingSchema }),
  }),
};
