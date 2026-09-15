use anchor_lang::prelude::*;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
#[program]
pub mod vault {
    pub fn withdraw(ctx: Context<W>, amount: u64) -> Result<()> {
        **ctx.accounts.vault.lamports.borrow_mut() -= amount;
        Ok(())
    }
}
