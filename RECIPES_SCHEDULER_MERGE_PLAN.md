# Recipes & Scheduler Merge Implementation Plan

## Overview
Merging the Scheduler functionality into the Recipes view to create a unified interface where recipes can optionally have schedules.

## Current State
- **RecipesView** (`/recipes`): Shows saved recipes with Use/Preview/Delete actions
- **SchedulesView** (`/schedules`): Shows scheduled jobs with cron expressions and management
- **Problem**: Schedules ARE recipes with timing, so having two separate views is redundant

## Implementation Status

### ✅ Completed
1. Created helper file `recipeScheduleHelpers.ts` with merge logic
2. Added schedule-related imports to RecipesView
3. Added schedule state variables to RecipesView
4. Backed up original RecipesView.tsx

### 🔄 In Progress - RecipesView.tsx Updates

#### Need to Add:
1. **Data Fetching** - Load both recipes AND schedules
   ```typescript
   const loadRecipesAndSchedules = async () => {
     const [recipes, schedules] = await Promise.all([
       listSavedRecipes(),
       listSchedules()
     ]);
     const merged = mergeRecipesWithSchedules(recipes, schedules);
     // Apply filter
     const filtered = filterRecipesByScheduleStatus(merged, filterType);
   };
   ```

2. **Filter Tabs** - Add UI to switch between All/Scheduled/Unscheduled
   ```tsx
   <div className="flex gap-2 mb-4">
     <Button onClick={() => setFilterType('all')}>All</Button>
     <Button onClick={() => setFilterType('scheduled')}>Scheduled</Button>
     <Button onClick={() => setFilterType('unscheduled')}>Unscheduled</Button>
   </div>
   ```

3. **Enhanced RecipeItem** - Show schedule info and add schedule actions
   - Badge showing if recipe is scheduled
   - Display cron expression (human-readable)
   - Show running status
   - Add "Schedule" button for unscheduled recipes
   - Add "Edit Schedule" / "Pause" / "Resume" / "Kill" for scheduled ones

4. **Schedule Handlers** - Add functions for schedule management
   - `handleCreateSchedule(recipe)` - Open CreateScheduleModal
   - `handleEditSchedule(schedule)` - Open EditScheduleModal
   - `handlePauseSchedule(scheduleId)`
   - `handleUnpauseSchedule(scheduleId)`
   - `handleDeleteSchedule(scheduleId)`
   - `handleKillRunningJob(scheduleId)`
   - `handleInspectRunningJob(scheduleId)`

5. **Modals** - Add schedule modals at bottom
   ```tsx
   <CreateScheduleModal 
     isOpen={showScheduleModal}
     onClose={() => setShowScheduleModal(false)}
     onSubmit={handleCreateScheduleSubmit}
     prefilledRecipePath={selectedRecipeForSchedule?.path}
   />
   <EditScheduleModal ... />
   ```

6. **Schedule Detail View** - Handle viewing schedule history
   ```tsx
   if (viewingScheduleId) {
     return <ScheduleDetailView 
       scheduleId={viewingScheduleId}
       onNavigateBack={() => setViewingScheduleId(null)}
     />;
   }
   ```

### 📋 TODO - Navigation Updates

1. **AppSidebar.tsx** - Remove "Scheduler" menu item
   - Remove the schedules navigation item from `menuItems` array
   - Keep only: Home, Chat, History, Recipes, Extensions, Settings

2. **App.tsx** - Remove `/schedules` route
   - Remove `<Route path="schedules" element={<SchedulesRoute />} />`
   - Remove `SchedulesRoute` component definition

3. **navigationUtils.ts** - Remove 'schedules' view type
   - Remove `'schedules'` from `View` type union
   - Remove `case 'schedules':` from `createNavigationHandler`

## File Structure

```
ui/desktop/src/components/
├── recipes/
│   ├── RecipesView.tsx          # Main unified view (IN PROGRESS)
│   ├── recipeScheduleHelpers.ts # Helper functions (✅ DONE)
│   ├── CreateRecipeForm.tsx     # Existing
│   └── ImportRecipeForm.tsx     # Existing
├── schedule/
│   ├── SchedulesView.tsx        # Will be deprecated/removed
│   ├── ScheduleDetailView.tsx   # Reused in RecipesView
│   ├── CreateScheduleModal.tsx  # Reused in RecipesView
│   └── EditScheduleModal.tsx    # Reused in RecipesView
└── GooseSidebar/
    └── AppSidebar.tsx           # Remove Scheduler item (TODO)
```

## Testing Checklist
- [ ] View all recipes with schedule indicators
- [ ] Filter by All/Scheduled/Unscheduled
- [ ] Create a schedule for an unscheduled recipe
- [ ] Edit an existing schedule
- [ ] Pause/Resume a schedule
- [ ] Kill a running scheduled job
- [ ] Inspect a running job
- [ ] Delete a schedule (recipe should remain)
- [ ] Delete a recipe that has a schedule
- [ ] View schedule detail/history
- [ ] Navigate back from schedule detail to recipes
- [ ] Verify Scheduler no longer appears in sidebar
- [ ] Verify /schedules route is removed

## Key Design Decisions

1. **Recipes are primary** - The view is called "Recipes" not "Recipes & Schedules"
2. **Schedules are optional** - A recipe can exist without a schedule
3. **Schedule info is inline** - No need to navigate away to see if recipe is scheduled
4. **Reuse existing components** - CreateScheduleModal, EditScheduleModal, ScheduleDetailView
5. **Keep schedule history** - ScheduleDetailView still accessible for viewing past runs

## Next Steps

1. Complete RecipesView.tsx implementation (large file, doing incrementally)
2. Update navigation (AppSidebar, App.tsx, navigationUtils.ts)
3. Test all functionality
4. Remove/deprecate SchedulesView.tsx
5. Update any documentation

## Notes
- RecipesView.tsx is 763 lines - implementing changes incrementally
- Backup created at RecipesView.tsx.backup
- Helper file created to keep main component cleaner
