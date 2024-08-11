use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token};

declare_id!("95g7UEjtYL7zguPZztyAi5cpRhmHHTk328dyWCjT2S7T");

#[program]
pub mod supercollabs_project {
    use super::*;

    pub fn create_project(
        ctx: Context<CreateProject>,
        name: String,
        description: String,
        total_allocation: u64,
    ) -> Result<()> {
        let project = &mut ctx.accounts.project;
        let clock = Clock::get()?;

        project.id = *project.to_account_info().key;
        project.name = name;
        project.description = description;
        project.state = ProjectState::Active;
        project.token_mint = ctx.accounts.token_mint.key();
        project.creator = ctx.accounts.creator.key();
        project.total_allocation = total_allocation;
        project.created_at = clock.unix_timestamp;

        // Initialize the token mint
        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::InitializeMint {
                mint: ctx.accounts.token_mint.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
        );
        token::initialize_mint(cpi_context, 9, &project.creator, Some(&project.creator))?;

        // Create the project token vault
        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::InitializeAccount {
                account: ctx.accounts.project_vault.to_account_info(),
                mint: ctx.accounts.token_mint.to_account_info(),
                authority: project.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
        );
        token::initialize_account(cpi_context)?;

        // Mint tokens to the project vault
        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.project_vault.to_account_info(),
                authority: ctx.accounts.creator.to_account_info(),
            },
        );
        token::mint_to(cpi_context, total_allocation)?;

        emit!(ProjectCreated {
            project_id: project.id,
            creator: project.creator,
            name: project.name.clone(),
            total_allocation: project.total_allocation,
        });

        Ok(())
    }

    pub fn update_project_state(ctx: Context<UpdateProjectState>, new_state: ProjectState) -> Result<()> {
        let project = &mut ctx.accounts.project;

        // Ensure the project is not already in the new state
        require!(project.state != new_state, ProjectError::InvalidStateTransition);

        let cloned_state = new_state.clone();
        project.state = new_state;

        emit!(ProjectStateUpdated {
            project_id: project.id,
            new_state: cloned_state,
        });

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(name: String, description: String, total_allocation: u64)]
pub struct CreateProject<'info> {
    #[account(
        init,
        payer = creator,
        space = Project::space(&name, &description),
    )]
    pub project: Account<'info, Project>,
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        init,
        payer = creator,
        mint::decimals = 9,
        mint::authority = creator.key(),
    )]
    pub token_mint: Account<'info, token::Mint>,
    #[account(
        init,
        payer = creator,
        token::mint = token_mint,
        token::authority = project,
    )]
    pub project_vault: Account<'info, token::TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateProjectState<'info> {
    #[account(
        mut,
        has_one = creator,
    )]
    pub project: Account<'info, Project>,
    pub creator: Signer<'info>,
}

#[account]
pub struct Project {
    pub id: Pubkey,
    pub name: String,
    pub description: String,
    pub state: ProjectState,
    pub token_mint: Pubkey,
    pub creator: Pubkey,
    pub total_allocation: u64,
    pub created_at: i64,
}

impl Project {
    fn space(name: &str, description: &str) -> usize {
        8 +  // discriminator
        32 + // id
        4 + name.len() + // name
        4 + description.len() + // description
        1 +  // state
        32 + // token_mint
        32 + // creator
        8 +  // total_allocation
        8    // created_at
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum ProjectState {
    Active,
    Completed,
    Cancelled,
}

#[error_code]
pub enum ProjectError {
    #[msg("Invalid state transition")]
    InvalidStateTransition,
}

#[event]
pub struct ProjectCreated {
    pub project_id: Pubkey,
    pub creator: Pubkey,
    pub name: String,
    pub total_allocation: u64,
}

#[event]
pub struct ProjectStateUpdated {
    pub project_id: Pubkey,
    pub new_state: ProjectState,
}